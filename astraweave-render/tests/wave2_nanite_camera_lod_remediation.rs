#![allow(unused, clippy::all)]
//! Wave 2 proactive remediation tests for nanite_visibility, camera, and lod_generator.
//!
//! These three modules have significant mutation surface with relatively few
//! existing tests:
//!   - nanite_visibility.rs: 126 mutants, 2 tests → BIGGEST GAP
//!   - camera.rs: 155 mutants, 7 tests
//!   - lod_generator.rs: 168 mutants, sparse coverage
//!
//! All tests are pure CPU math — no GPU/wgpu context needed.

#![cfg(feature = "nanite")]

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::nanite_visibility::{Frustum, GpuMeshlet, LODSelector};
use glam::{Mat4, Vec3, Vec4};

// ══════════════════════════════════════════════════════════════════════════════
// Frustum::from_matrix — plane extraction (Gribb-Hartmann)
// ══════════════════════════════════════════════════════════════════════════════

fn ortho_frustum() -> Frustum {
    Frustum::from_matrix(Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0))
}

fn perspective_frustum() -> Frustum {
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_3, 16.0 / 9.0, 0.1, 1000.0);
    let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    Frustum::from_matrix(proj * view)
}

#[test]
fn frustum_ortho_plane_count() {
    let f = ortho_frustum();
    assert_eq!(f.planes.len(), 6);
    for plane in &f.planes {
        // Each plane normal should be normalized (length ≈ 1)
        let normal_len = Vec3::new(plane.x, plane.y, plane.z).length();
        assert!(
            (normal_len - 1.0).abs() < 0.01,
            "Plane normal should be normalized, got length={normal_len}"
        );
    }
}

#[test]
fn frustum_perspective_planes_normalized() {
    let f = perspective_frustum();
    for (i, plane) in f.planes.iter().enumerate() {
        let len = Vec3::new(plane.x, plane.y, plane.z).length();
        assert!(
            (len - 1.0).abs() < 0.01,
            "Perspective plane {i} normal len={len} should be ~1.0"
        );
    }
}

#[test]
fn frustum_ortho_different_from_perspective() {
    let fo = ortho_frustum();
    let fp = perspective_frustum();
    // The planes should differ
    let same_count = fo
        .planes
        .iter()
        .zip(fp.planes.iter())
        .filter(|(a, b)| (**a - **b).length() < 0.001)
        .count();
    assert!(same_count < 6, "Ortho and perspective should differ");
}

// ══════════════════════════════════════════════════════════════════════════════
// Frustum::test_aabb — AABB vs frustum containment
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn aabb_at_origin_inside_ortho() {
    let f = ortho_frustum();
    assert!(f.test_aabb(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0)));
}

#[test]
fn aabb_far_outside_ortho() {
    let f = ortho_frustum();
    assert!(!f.test_aabb(Vec3::new(50.0, 50.0, 50.0), Vec3::new(60.0, 60.0, 60.0)));
}

#[test]
fn aabb_partially_inside_ortho() {
    let f = ortho_frustum();
    // AABB overlaps the right boundary
    assert!(f.test_aabb(Vec3::new(5.0, -1.0, -1.0), Vec3::new(15.0, 1.0, 1.0)));
}

#[test]
fn aabb_just_outside_left_ortho() {
    let f = ortho_frustum();
    // AABB completely left of -10
    assert!(!f.test_aabb(Vec3::new(-20.0, -1.0, -1.0), Vec3::new(-11.0, 1.0, 1.0)));
}

#[test]
fn aabb_outside_on_multiple_axes() {
    let f = ortho_frustum();
    // Outside on both X and Y simultaneously
    assert!(!f.test_aabb(Vec3::new(15.0, 15.0, -5.0), Vec3::new(20.0, 20.0, -3.0)));
}

#[test]
fn aabb_beyond_far_plane() {
    let f = ortho_frustum();
    // Beyond the far plane
    assert!(!f.test_aabb(Vec3::new(-1.0, -1.0, -200.0), Vec3::new(1.0, 1.0, -150.0)));
}

#[test]
fn aabb_at_frustum_boundary_x() {
    let f = ortho_frustum();
    // AABB touching the right boundary
    assert!(f.test_aabb(Vec3::new(9.0, -1.0, -1.0), Vec3::new(10.0, 1.0, 1.0)));
}

#[test]
fn aabb_min_max_axes_independent() {
    let f = ortho_frustum();
    // Each axis boundary checked independently
    // Outside on X but inside on Y and Z — should fail
    assert!(!f.test_aabb(Vec3::new(20.0, -1.0, -1.0), Vec3::new(30.0, 1.0, 1.0)));
    // Inside on X but outside on Y — should fail
    assert!(!f.test_aabb(Vec3::new(-1.0, 20.0, -1.0), Vec3::new(1.0, 30.0, 1.0)));
}

// ══════════════════════════════════════════════════════════════════════════════
// Frustum::test_sphere
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn sphere_at_origin_inside() {
    let f = ortho_frustum();
    assert!(f.test_sphere(Vec3::ZERO, 1.0));
}

#[test]
fn sphere_far_outside() {
    let f = ortho_frustum();
    assert!(!f.test_sphere(Vec3::new(100.0, 100.0, 100.0), 1.0));
}

#[test]
fn sphere_radius_overlaps_boundary() {
    let f = ortho_frustum();
    // Center just outside, but radius reaches inside
    assert!(f.test_sphere(Vec3::new(11.0, 0.0, -5.0), 2.0));
}

#[test]
fn sphere_exactly_outside_with_too_small_radius() {
    let f = ortho_frustum();
    // Center at 12, radius 1 → furthest reach is 11, but boundary is 10
    assert!(!f.test_sphere(Vec3::new(12.0, 0.0, -5.0), 1.0));
}

#[test]
fn sphere_zero_radius_at_origin() {
    let f = ortho_frustum();
    assert!(f.test_sphere(Vec3::ZERO, 0.0));
}

#[test]
fn sphere_large_radius_always_inside() {
    let f = ortho_frustum();
    // Very large sphere should always intersect
    assert!(f.test_sphere(Vec3::new(0.0, 0.0, -50.0), 1000.0));
}

// ══════════════════════════════════════════════════════════════════════════════
// LODSelector
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn lod_selector_new_defaults() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    assert_eq!(sel.screen_height, 1080.0);
    assert_eq!(sel.fov, std::f32::consts::FRAC_PI_3);
    assert_eq!(sel.lod_bias, 1.0);
}

#[test]
fn lod_selector_close_object_lod0() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -5.0), 1.0, 0.1, Vec3::ZERO, 4);
    assert_eq!(lod, 0, "Close object should use LOD 0");
}

#[test]
fn lod_selector_far_object_higher_lod() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -500.0), 1.0, 100.0, Vec3::ZERO, 4);
    assert!(lod > 0, "Far object should use LOD > 0, got {lod}");
}

#[test]
fn lod_selector_max_lod_capped() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -10000.0), 0.1, 100.0, Vec3::ZERO, 3);
    assert!(lod <= 3, "LOD should not exceed max_lod=3, got {lod}");
}

#[test]
fn lod_selector_distance_zero_returns_lod0() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    // Object at camera position: projected_size = screen_height, should be LOD 0
    let lod = sel.select_lod(Vec3::ZERO, 1.0, 0.1, Vec3::ZERO, 4);
    assert_eq!(lod, 0);
}

#[test]
fn lod_selector_large_error_threshold_prefers_lod0() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    // Large error threshold means we tolerate more → LOD 0 preferred
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -50.0), 5.0, 0.001, Vec3::ZERO, 4);
    assert_eq!(lod, 0);
}

#[test]
fn lod_selector_lod_bias_higher_prefers_lower_detail() {
    let mut sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_3);
    let lod_base = sel.select_lod(Vec3::new(0.0, 0.0, -100.0), 1.0, 50.0, Vec3::ZERO, 4);

    sel.lod_bias = 5.0;
    let lod_biased = sel.select_lod(Vec3::new(0.0, 0.0, -100.0), 1.0, 50.0, Vec3::ZERO, 4);
    assert!(
        lod_biased >= lod_base,
        "Higher bias should prefer same or lower detail: base={lod_base}, biased={lod_biased}"
    );
}

#[test]
fn lod_selector_different_screen_heights() {
    // Higher resolution → larger projected size → more likely LOD 0
    let sel_4k = LODSelector::new(2160.0, std::f32::consts::FRAC_PI_3);
    let sel_720 = LODSelector::new(720.0, std::f32::consts::FRAC_PI_3);

    let pos = Vec3::new(0.0, 0.0, -200.0);
    let lod_4k = sel_4k.select_lod(pos, 1.0, 50.0, Vec3::ZERO, 4);
    let lod_720 = sel_720.select_lod(pos, 1.0, 50.0, Vec3::ZERO, 4);

    assert!(
        lod_720 >= lod_4k,
        "Lower resolution should use same/higher LOD: 4k={lod_4k}, 720={lod_720}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// GpuMeshlet struct layout
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_meshlet_pod_zeroed() {
    let m = GpuMeshlet {
        bounds_min: [0.0; 3],
        vertex_offset: 0,
        bounds_max: [0.0; 3],
        vertex_count: 0,
        cone_apex: [0.0; 3],
        triangle_offset: 0,
        cone_axis: [0.0; 3],
        triangle_count: 0,
        cone_cutoff: 0.0,
        lod_level: 0,
        lod_error: 0.0,
        material_id: 0,
    };
    assert_eq!(m.bounds_min, [0.0; 3]);
    assert_eq!(m.vertex_count, 0);
}

#[test]
fn gpu_meshlet_size() {
    assert_eq!(
        std::mem::size_of::<GpuMeshlet>(),
        80,
        "GpuMeshlet should be 80 bytes (with padding)"
    );
}

#[test]
fn gpu_meshlet_field_values() {
    let m = GpuMeshlet {
        bounds_min: [-1.0, -2.0, -3.0],
        vertex_offset: 100,
        bounds_max: [1.0, 2.0, 3.0],
        vertex_count: 64,
        cone_apex: [0.5, 0.5, 0.5],
        triangle_offset: 200,
        cone_axis: [0.0, 1.0, 0.0],
        triangle_count: 128,
        cone_cutoff: 0.3,
        lod_level: 2,
        lod_error: 0.01,
        material_id: 5,
    };
    assert_eq!(m.bounds_min[0], -1.0);
    assert_eq!(m.bounds_max[2], 3.0);
    assert_eq!(m.vertex_offset, 100);
    assert_eq!(m.triangle_count, 128);
    assert_eq!(m.cone_cutoff, 0.3);
    assert_eq!(m.lod_level, 2);
    assert_eq!(m.material_id, 5);
}

// ══════════════════════════════════════════════════════════════════════════════
// Frustum::from_matrix with identity — all planes at ±1
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn frustum_identity_matrix() {
    let f = Frustum::from_matrix(Mat4::IDENTITY);
    // With identity VP matrix, points at origin should be inside
    assert!(f.test_aabb(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5)));
}

#[test]
fn frustum_from_perspective_distant_object_culled() {
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let f = Frustum::from_matrix(proj * view);

    // Far behind the camera: should be culled
    assert!(!f.test_aabb(Vec3::new(-1.0, -1.0, 5.0), Vec3::new(1.0, 1.0, 10.0)));
}

#[test]
fn frustum_perspective_aabb_near_inside() {
    let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_4, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
    let f = Frustum::from_matrix(proj * view);

    // Object just in front of camera
    assert!(f.test_aabb(Vec3::new(-1.0, -1.0, -5.0), Vec3::new(1.0, 1.0, -1.0)));
}
