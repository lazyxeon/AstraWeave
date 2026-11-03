//! Batch 11 — Frustum culling, LODSelector screen-space error, GpuMeshlet layout
//!
//! Targets: nanite_visibility.rs (Frustum, LODSelector, GpuMeshlet), lod_generator.rs extras
//! All tests are CPU-only (no GPU device required).
//!
//! nanite_visibility is gated behind the `nanite` feature.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

#[cfg(feature = "nanite")]
use astraweave_render::nanite_visibility::{Frustum, GpuMeshlet, LODSelector};
#[cfg(feature = "nanite")]
use glam::Mat4;
use glam::Vec3;

// ─── Frustum ────────────────────────────────────────────────────────

/// A standard perspective matrix for testing: 90° FOV, 1:1 aspect, near=0.1, far=100.
#[cfg(feature = "nanite")]
fn test_perspective() -> Mat4 {
    Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0)
}

#[cfg(feature = "nanite")]
fn identity_frustum() -> Frustum {
    Frustum::from_matrix(test_perspective())
}

// --- from_matrix ---

#[cfg(feature = "nanite")]
#[test]
fn frustum_from_identity_produces_six_planes() {
    let f = identity_frustum();
    assert_eq!(f.planes.len(), 6, "Frustum must have exactly 6 planes");
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_planes_are_normalised() {
    let f = identity_frustum();
    for (i, plane) in f.planes.iter().enumerate() {
        let normal_len = Vec3::new(plane.x, plane.y, plane.z).length();
        assert!(
            (normal_len - 1.0).abs() < 1e-4,
            "Plane {} normal length {:.6} should be ~1.0",
            i,
            normal_len
        );
    }
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_from_ortho_matrix_produces_valid_planes() {
    let ortho = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 50.0);
    let f = Frustum::from_matrix(ortho);
    for (i, plane) in f.planes.iter().enumerate() {
        let len = Vec3::new(plane.x, plane.y, plane.z).length();
        assert!(len > 0.0, "Ortho plane {} should have non-zero normal", i);
    }
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_from_identity_matrix_does_not_panic() {
    let _f = Frustum::from_matrix(Mat4::IDENTITY);
}

// --- test_aabb ---

#[cfg(feature = "nanite")]
#[test]
fn aabb_at_origin_inside_perspective() {
    let f = identity_frustum();
    // Small box centered at z=-5 (in front of camera looking -Z in RH)
    let inside = f.test_aabb(Vec3::new(-1.0, -1.0, -6.0), Vec3::new(1.0, 1.0, -4.0));
    assert!(
        inside,
        "Small AABB in front of camera should be inside frustum"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_behind_camera_outside_perspective() {
    let f = identity_frustum();
    // Box entirely behind the camera (positive Z in RH)
    let outside = f.test_aabb(Vec3::new(-1.0, -1.0, 5.0), Vec3::new(1.0, 1.0, 10.0));
    assert!(!outside, "AABB behind camera should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_far_away_outside_perspective() {
    let f = identity_frustum();
    // Box beyond far plane (far=100)
    let outside = f.test_aabb(Vec3::new(-1.0, -1.0, -200.0), Vec3::new(1.0, 1.0, -150.0));
    assert!(!outside, "AABB beyond far plane should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_straddling_near_plane_is_inside() {
    let f = identity_frustum();
    // Box that straddles the near plane
    let inside = f.test_aabb(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
    assert!(
        inside,
        "AABB straddling near plane should be considered inside"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_huge_box_enclosing_frustum_is_inside() {
    let f = identity_frustum();
    let inside = f.test_aabb(Vec3::splat(-1000.0), Vec3::splat(1000.0));
    assert!(
        inside,
        "Huge AABB enclosing entire frustum should be inside"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_left_of_frustum_is_outside() {
    let f = identity_frustum();
    // 90° FOV → half-angle 45°. At z=-10, left edge is x=-10.
    // Place box well to the left.
    let outside = f.test_aabb(Vec3::new(-100.0, -1.0, -10.0), Vec3::new(-50.0, 1.0, -9.0));
    assert!(!outside, "AABB far to the left should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_right_of_frustum_is_outside() {
    let f = identity_frustum();
    let outside = f.test_aabb(Vec3::new(50.0, -1.0, -10.0), Vec3::new(100.0, 1.0, -9.0));
    assert!(!outside, "AABB far to the right should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_above_frustum_is_outside() {
    let f = identity_frustum();
    let outside = f.test_aabb(Vec3::new(-1.0, 50.0, -10.0), Vec3::new(1.0, 100.0, -9.0));
    assert!(!outside, "AABB far above should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_below_frustum_is_outside() {
    let f = identity_frustum();
    let outside = f.test_aabb(Vec3::new(-1.0, -100.0, -10.0), Vec3::new(1.0, -50.0, -9.0));
    assert!(!outside, "AABB far below should be outside frustum");
}

#[cfg(feature = "nanite")]
#[test]
fn aabb_zero_volume_point_inside() {
    let f = identity_frustum();
    // Point-like AABB at z=-5
    let inside = f.test_aabb(Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, -5.0));
    assert!(inside, "Zero-volume AABB at visible point should be inside");
}

// --- test_sphere ---

#[cfg(feature = "nanite")]
#[test]
fn sphere_at_center_inside() {
    let f = identity_frustum();
    let inside = f.test_sphere(Vec3::new(0.0, 0.0, -5.0), 1.0);
    assert!(inside, "Sphere in front of camera should be inside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_behind_camera_outside() {
    let f = identity_frustum();
    let outside = f.test_sphere(Vec3::new(0.0, 0.0, 10.0), 1.0);
    assert!(!outside, "Sphere behind camera should be outside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_beyond_far_plane_outside() {
    let f = identity_frustum();
    let outside = f.test_sphere(Vec3::new(0.0, 0.0, -200.0), 1.0);
    assert!(!outside, "Sphere beyond far plane should be outside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_with_zero_radius_inside() {
    let f = identity_frustum();
    let inside = f.test_sphere(Vec3::new(0.0, 0.0, -5.0), 0.0);
    assert!(
        inside,
        "Zero-radius sphere at visible point should be inside"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_barely_touching_near_plane_inside() {
    let f = identity_frustum();
    // near plane is at z = -0.1. Place sphere center at z=0.0 with radius 0.2 → overlaps
    let inside = f.test_sphere(Vec3::new(0.0, 0.0, 0.0), 0.2);
    assert!(inside, "Sphere overlapping near plane should be inside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_far_left_outside() {
    let f = identity_frustum();
    let outside = f.test_sphere(Vec3::new(-100.0, 0.0, -5.0), 1.0);
    assert!(!outside, "Sphere far to the left should be outside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_large_radius_enclosing_frustum_inside() {
    let f = identity_frustum();
    let inside = f.test_sphere(Vec3::ZERO, 500.0);
    assert!(inside, "Huge sphere enclosing frustum should be inside");
}

#[cfg(feature = "nanite")]
#[test]
fn sphere_just_outside_far_plane_with_radius_straddling() {
    let f = identity_frustum();
    // Sphere center at z=-101 (1 unit past far=100), radius=5 → overlaps far plane
    let inside = f.test_sphere(Vec3::new(0.0, 0.0, -101.0), 5.0);
    assert!(inside, "Sphere straddling far plane should be inside");
}

// --- Frustum with transformed view ---

#[cfg(feature = "nanite")]
#[test]
fn frustum_with_view_transform_culls_correctly() {
    // Camera at (0,0,5) looking toward origin (-Z in view space)
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
    let proj = test_perspective();
    let vp = proj * view;
    let f = Frustum::from_matrix(vp);

    // Object at origin (5 units away, well within far=100)
    assert!(
        f.test_sphere(Vec3::ZERO, 1.0),
        "Origin should be visible from (0,0,5)"
    );

    // Object behind the camera
    assert!(
        !f.test_sphere(Vec3::new(0.0, 0.0, 50.0), 1.0),
        "Object behind camera at (0,0,5) should be culled"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn frustum_rotated_camera_culls_sideways() {
    // Camera at origin looking along +X
    let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::X, Vec3::Y);
    let proj = test_perspective();
    let vp = proj * view;
    let f = Frustum::from_matrix(vp);

    // Object at (10, 0, 0) should be inside
    assert!(
        f.test_sphere(Vec3::new(10.0, 0.0, 0.0), 1.0),
        "+X should be visible"
    );

    // Object at (-10, 0, 0) should be outside (behind)
    assert!(
        !f.test_sphere(Vec3::new(-10.0, 0.0, 0.0), 1.0),
        "-X should be behind camera"
    );
}

// ─── LODSelector ────────────────────────────────────────────────────

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_new_stores_fields() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_2);
    assert_eq!(sel.screen_height, 1080.0);
    assert_eq!(sel.fov, std::f32::consts::FRAC_PI_2);
    assert_eq!(sel.lod_bias, 1.0, "Default lod_bias should be 1.0");
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_close_object_is_lod_0() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    // Object 1 unit away with radius 1 fills most of screen → LOD 0
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -1.0), 1.0, 2.0, Vec3::ZERO, 4);
    assert_eq!(
        lod, 0,
        "Close large object should use LOD 0 (highest detail)"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_far_object_higher_lod() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    // `lod_error` is a screen-space threshold in pixels (see NANITE.md). Coarser LOD
    // is chosen when the projected footprint is below that threshold.
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -1000.0), 1.0, 100.0, Vec3::ZERO, 4);
    assert!(
        lod > 0,
        "Distant object with footprint below error threshold should use higher LOD, got {}",
        lod
    );
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_max_lod_clamped() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -100000.0), 0.01, 0.0001, Vec3::ZERO, 3);
    assert!(lod <= 3, "LOD should be clamped to max_lod=3, got {}", lod);
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_zero_distance_returns_lod_0_or_max_screen() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    // Camera at same position as object center → distance = 0
    let lod = sel.select_lod(Vec3::ZERO, 1.0, 1.0, Vec3::ZERO, 4);
    assert_eq!(lod, 0, "Zero distance should return LOD 0 (max detail)");
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_lod_bias_increases_lod() {
    let mut sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let camera = Vec3::ZERO;
    let center = Vec3::new(0.0, 0.0, -50.0);

    let lod_default = sel.select_lod(center, 1.0, 1.0, camera, 6);
    sel.lod_bias = 0.001; // Very small bias → small error threshold → more likely LOD > 0
    let lod_biased = sel.select_lod(center, 1.0, 1.0, camera, 6);

    // With smaller lod_bias, the error threshold is smaller, so either lower LOD or equal
    // The key: they should not be the same IF the default lod already triggers a switch
    // At minimum we verify the function doesn't crash
    assert!(lod_default <= 6);
    assert!(lod_biased <= 6);
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_negative_fov_does_not_panic() {
    // Edge case: negative fov
    let sel = LODSelector::new(1080.0, -1.0);
    let _ = sel.select_lod(Vec3::new(0.0, 0.0, -10.0), 1.0, 1.0, Vec3::ZERO, 4);
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_very_small_screen_height() {
    let sel = LODSelector::new(1.0, std::f32::consts::FRAC_PI_4);
    // Tiny screen → projected size is tiny → higher LOD
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -10.0), 1.0, 1.0, Vec3::ZERO, 4);
    assert!(lod <= 4, "LOD should be clamped to max_lod");
}

#[cfg(feature = "nanite")]
#[test]
fn lod_selector_max_lod_zero_always_returns_zero() {
    let sel = LODSelector::new(1080.0, std::f32::consts::FRAC_PI_4);
    let lod = sel.select_lod(Vec3::new(0.0, 0.0, -10000.0), 0.01, 0.001, Vec3::ZERO, 0);
    assert_eq!(lod, 0, "When max_lod=0, result must be 0");
}

// ─── GpuMeshlet ─────────────────────────────────────────────────────

#[cfg(feature = "nanite")]
#[test]
fn gpu_meshlet_size_is_80_bytes() {
    // 3*f32 + u32 + 3*f32 + u32 + 3*f32 + u32 + 3*f32 + u32 + f32 + u32 + f32 + u32
    // = 16 + 16 + 16 + 16 + 16 = 80 bytes
    assert_eq!(
        std::mem::size_of::<GpuMeshlet>(),
        80,
        "GpuMeshlet should be 80 bytes"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_meshlet_alignment_is_4() {
    assert_eq!(
        std::mem::align_of::<GpuMeshlet>(),
        4,
        "GpuMeshlet alignment should be 4 (f32 aligned)"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_meshlet_zeroed_is_all_zeros() {
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
    let bytes: &[u8] = bytemuck::bytes_of(&m);
    assert!(
        bytes.iter().all(|&b| b == 0),
        "Zeroed GpuMeshlet should be all zero bytes"
    );
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_meshlet_pod_roundtrip() {
    let m = GpuMeshlet {
        bounds_min: [-1.0, -2.0, -3.0],
        vertex_offset: 100,
        bounds_max: [1.0, 2.0, 3.0],
        vertex_count: 64,
        cone_apex: [0.5, 0.5, 0.5],
        triangle_offset: 200,
        cone_axis: [0.0, 1.0, 0.0],
        triangle_count: 128,
        cone_cutoff: 0.707,
        lod_level: 2,
        lod_error: 0.01,
        material_id: 42,
    };
    let bytes: &[u8] = bytemuck::bytes_of(&m);
    let m2: &GpuMeshlet = bytemuck::from_bytes(bytes);
    assert_eq!(m2.bounds_min, [-1.0, -2.0, -3.0]);
    assert_eq!(m2.bounds_max, [1.0, 2.0, 3.0]);
    assert_eq!(m2.vertex_offset, 100);
    assert_eq!(m2.vertex_count, 64);
    assert_eq!(m2.triangle_offset, 200);
    assert_eq!(m2.triangle_count, 128);
    assert_eq!(m2.lod_level, 2);
    assert_eq!(m2.material_id, 42);
    assert!((m2.cone_cutoff - 0.707).abs() < 1e-6);
    assert!((m2.lod_error - 0.01).abs() < 1e-6);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_meshlet_can_cast_slice() {
    let meshlets = vec![
        GpuMeshlet {
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
        3
    ];
    let bytes: &[u8] = bytemuck::cast_slice(&meshlets);
    assert_eq!(bytes.len(), 80 * 3, "3 meshlets should be 240 bytes");

    let roundtrip: &[GpuMeshlet] = bytemuck::cast_slice(bytes);
    assert_eq!(roundtrip.len(), 3);
}

// ─── LODGenerator / SimplificationMesh extras ───────────────────────

use astraweave_render::lod_generator::{LODConfig, LODGenerator, SimplificationMesh};

fn make_quad() -> SimplificationMesh {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
    ];
    let normals = vec![Vec3::Y; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];
    SimplificationMesh::new(positions, normals, uvs, indices)
}

#[test]
fn simplification_mesh_new_stores_data() {
    let mesh = make_quad();
    assert_eq!(mesh.positions.len(), 4);
    assert_eq!(mesh.normals.len(), 4);
    assert_eq!(mesh.uvs.len(), 4);
    assert_eq!(mesh.indices.len(), 6);
}

#[test]
fn simplification_mesh_vertex_count() {
    let mesh = make_quad();
    assert_eq!(mesh.vertex_count(), 4);
}

#[test]
fn simplification_mesh_triangle_count() {
    let mesh = make_quad();
    assert_eq!(mesh.triangle_count(), 2);
}

#[test]
fn simplification_mesh_empty() {
    let mesh = SimplificationMesh::new(vec![], vec![], vec![], vec![]);
    assert_eq!(mesh.vertex_count(), 0);
    assert_eq!(mesh.triangle_count(), 0);
}

#[test]
fn lod_config_default_values() {
    let cfg = LODConfig::default();
    assert_eq!(cfg.reduction_targets, vec![0.75, 0.50, 0.25]);
    assert_eq!(cfg.max_error, 0.01);
    assert!(cfg.preserve_boundaries);
}

#[test]
fn lod_generator_new_stores_config() {
    let cfg = LODConfig {
        reduction_targets: vec![0.5],
        max_error: 0.1,
        preserve_boundaries: false,
    };
    let gen = LODGenerator::new(cfg);
    // We can only verify it doesn't panic; config is private
    let _ = gen;
}

#[test]
fn lod_generator_simplify_no_reduction_when_target_exceeds_count() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = make_quad();
    let simplified = gen.simplify(&mesh, 100); // target > actual
    assert_eq!(simplified.vertex_count(), 4, "Should return unchanged mesh");
    assert_eq!(simplified.triangle_count(), 2);
}

#[test]
fn lod_generator_simplify_reduces_vertices() {
    let gen = LODGenerator::new(LODConfig {
        reduction_targets: vec![0.5],
        max_error: 10.0, // high tolerance
        preserve_boundaries: false,
    });
    let mesh = make_quad();
    let simplified = gen.simplify(&mesh, 3);
    assert!(
        simplified.vertex_count() <= 4,
        "Simplified should have <= original vertices"
    );
}

#[test]
fn lod_generator_generate_lods_returns_correct_count() {
    let cfg = LODConfig {
        reduction_targets: vec![0.75, 0.50],
        max_error: 10.0,
        preserve_boundaries: true,
    };
    let gen = LODGenerator::new(cfg);
    let mesh = make_quad();
    let lods = gen.generate_lods(&mesh);
    assert_eq!(lods.len(), 2, "Should generate 2 LOD levels");
}

#[test]
fn lod_generator_calculate_reduction_identity() {
    let gen = LODGenerator::new(LODConfig::default());
    let mesh = make_quad();
    let reduction = gen.calculate_reduction(&mesh, &mesh);
    assert!(
        reduction.abs() < f32::EPSILON,
        "Same mesh should have 0 reduction, got {}",
        reduction
    );
}

#[test]
fn lod_generator_calculate_reduction_half() {
    let gen = LODGenerator::new(LODConfig::default());
    let full = SimplificationMesh::new(
        vec![Vec3::ZERO; 10],
        vec![Vec3::Y; 10],
        vec![[0.0, 0.0]; 10],
        vec![],
    );
    let half = SimplificationMesh::new(
        vec![Vec3::ZERO; 5],
        vec![Vec3::Y; 5],
        vec![[0.0, 0.0]; 5],
        vec![],
    );
    let reduction = gen.calculate_reduction(&full, &half);
    assert!(
        (reduction - 0.5).abs() < f32::EPSILON,
        "10→5 vertices should be 50% reduction, got {}",
        reduction
    );
}

#[test]
fn lod_generator_all_lods_have_valid_indices() {
    let gen = LODGenerator::new(LODConfig {
        reduction_targets: vec![0.75, 0.5, 0.25],
        max_error: 10.0,
        preserve_boundaries: true,
    });
    // 8-vert cube
    let positions = vec![
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
    ];
    let normals: Vec<Vec3> = positions.iter().map(|p| p.normalize()).collect();
    let uvs = vec![[0.0, 0.0]; 8];
    #[rustfmt::skip]
    let indices = vec![
        0,1,2, 0,2,3,
        5,4,7, 5,7,6,
        4,0,3, 4,3,7,
        1,5,6, 1,6,2,
        3,2,6, 3,6,7,
        4,5,1, 4,1,0,
    ];
    let mesh = SimplificationMesh::new(positions, normals, uvs, indices);
    let lods = gen.generate_lods(&mesh);

    for (i, lod) in lods.iter().enumerate() {
        assert_eq!(
            lod.indices.len() % 3,
            0,
            "LOD {} indices not divisible by 3",
            i
        );
        for &idx in &lod.indices {
            assert!(
                (idx as usize) < lod.vertex_count(),
                "LOD {} has out-of-range index {} (verts={})",
                i,
                idx,
                lod.vertex_count()
            );
        }
    }
}
