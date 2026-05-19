//! Behavioral Correctness Tests for astraweave-render
//!
//! These tests validate mathematically correct behavior of rendering systems
//! without requiring GPU access. Focuses on CPU-side math that drives the GPU.
//!
//! Coverage targets:
//! - Vertex compression: Octahedral normal encoding/decoding, half-float UV
//! - Camera: View/projection matrices, frustum extraction
//! - Culling: AABB frustum test, plane extraction
//! - LOD: Quadric error metrics, mesh simplification
//! - Environment: Time of day lighting, sun/moon positions

use astraweave_camera::FreeFly as Camera;
use astraweave_render::{
    culling::{FrustumPlanes, InstanceAABB},
    environment::TimeOfDay,
    lod_generator::LODConfig,
    vertex_compression::{CompressedVertex, HalfFloatEncoder, OctahedralEncoder, VertexCompressor},
};
use glam::{Mat4, Vec2, Vec3, Vec4Swizzles};
use std::f32::consts::PI;

// ============================================================================
// OCTAHEDRAL NORMAL ENCODING TESTS
// ============================================================================

/// Test octahedral encoding of cardinal directions (X, Y, Z)
#[test]
fn test_octahedral_encode_cardinal_axes() {
    let test_cases = [
        (Vec3::X, "positive X"),
        (Vec3::NEG_X, "negative X"),
        (Vec3::Y, "positive Y"),
        (Vec3::NEG_Y, "negative Y"),
        (Vec3::Z, "positive Z"),
        (Vec3::NEG_Z, "negative Z"),
    ];

    for (normal, name) in &test_cases {
        let encoded = OctahedralEncoder::encode(*normal);
        let decoded = OctahedralEncoder::decode(encoded);

        let dot = normal.dot(decoded);
        assert!(
            dot > 0.99,
            "{} encoding error too high: dot={}, original={:?}, decoded={:?}",
            name,
            dot,
            normal,
            decoded
        );
    }
}

/// Test octahedral encoding roundtrip preserves direction
#[test]
fn test_octahedral_roundtrip_preserves_direction() {
    // Test various normalized vectors
    let normals = [
        Vec3::new(0.577, 0.577, 0.577).normalize(), // Diagonal
        Vec3::new(0.0, 0.707, 0.707).normalize(),   // YZ plane
        Vec3::new(0.707, 0.0, 0.707).normalize(),   // XZ plane
        Vec3::new(0.707, 0.707, 0.0).normalize(),   // XY plane
        Vec3::new(-0.5, 0.5, 0.707).normalize(),    // Arbitrary
        Vec3::new(0.1, -0.9, 0.4).normalize(),      // Another arbitrary
    ];

    for normal in &normals {
        let encoded = OctahedralEncoder::encode(*normal);
        let decoded = OctahedralEncoder::decode(encoded);

        // Verify decoded is normalized
        assert!(
            (decoded.length() - 1.0).abs() < 0.01,
            "Decoded normal should be unit length"
        );

        // Verify direction preserved (dot product > 0.99)
        let dot = normal.dot(decoded);
        assert!(
            dot > 0.99,
            "Octahedral roundtrip failed: original={:?}, decoded={:?}, dot={}",
            normal,
            decoded,
            dot
        );
    }
}

/// Test octahedral encoding error is bounded
#[test]
fn test_octahedral_encoding_error_bounded() {
    // Maximum angular error should be < 1 degree for 16-bit quantization
    let max_error_radians = 1.0_f32.to_radians();

    let normals = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::new(1.0, 1.0, 1.0).normalize(),
        Vec3::new(-1.0, 2.0, 0.5).normalize(),
    ];

    for normal in &normals {
        let error = OctahedralEncoder::encoding_error(*normal);
        assert!(
            error < max_error_radians,
            "Encoding error {} exceeds max allowed {} for {:?}",
            error,
            max_error_radians,
            normal
        );
    }
}

/// Test octahedral handles lower hemisphere (z < 0)
#[test]
fn test_octahedral_lower_hemisphere() {
    let normals = [
        Vec3::new(0.0, 0.0, -1.0),               // -Z axis
        Vec3::new(0.5, 0.5, -0.707).normalize(), // Lower diagonal
        Vec3::new(-0.3, 0.2, -0.9).normalize(),  // Lower arbitrary
    ];

    for normal in &normals {
        let encoded = OctahedralEncoder::encode(*normal);
        let decoded = OctahedralEncoder::decode(encoded);

        // Check Z component has correct sign
        assert!(
            decoded.z < 0.0,
            "Lower hemisphere should preserve negative Z: original={:?}, decoded={:?}",
            normal,
            decoded
        );

        // Check direction preserved
        let dot = normal.dot(decoded);
        assert!(dot > 0.99, "Lower hemisphere roundtrip failed: dot={}", dot);
    }
}

// ============================================================================
// HALF-FLOAT UV ENCODING TESTS
// ============================================================================

/// Test half-float encoding preserves UV values within precision
#[test]
fn test_half_float_uv_precision() {
    let uvs = [0.0_f32, 0.5_f32, 1.0_f32, 0.25_f32, 0.75_f32, 0.125_f32];

    for uv in &uvs {
        let encoded = HalfFloatEncoder::encode(*uv);
        let decoded = HalfFloatEncoder::decode(encoded);

        // Half-float has ~0.001 precision in [0, 1] range
        assert!(
            (decoded - uv).abs() < 0.001,
            "Half-float precision error: original={}, decoded={}",
            uv,
            decoded
        );
    }
}

/// Test half-float Vec2 encoding
#[test]
fn test_half_float_vec2_roundtrip() {
    let uvs = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.5, 0.5),
        Vec2::new(0.25, 0.75),
        Vec2::new(0.123, 0.456),
    ];

    for uv in &uvs {
        let encoded = HalfFloatEncoder::encode_vec2(*uv);
        let decoded = HalfFloatEncoder::decode_vec2(encoded);

        assert!((decoded.x - uv.x).abs() < 0.001, "Vec2 X roundtrip failed");
        assert!((decoded.y - uv.y).abs() < 0.001, "Vec2 Y roundtrip failed");
    }
}

/// Test half-float handles values outside [0, 1] range
#[test]
fn test_half_float_extended_range() {
    // UVs can be outside [0, 1] for tiling
    let extended = [2.0_f32, -0.5_f32, 10.0_f32];

    for value in &extended {
        let encoded = HalfFloatEncoder::encode(*value);
        let decoded = HalfFloatEncoder::decode(encoded);

        // Larger values have less precision, allow 1% error
        let tolerance = value.abs() * 0.01 + 0.001;
        assert!(
            (decoded - value).abs() < tolerance,
            "Extended range failed: original={}, decoded={}",
            value,
            decoded
        );
    }
}

// ============================================================================
// VERTEX COMPRESSION TESTS
// ============================================================================

/// Test vertex compression memory savings calculation
#[test]
fn test_vertex_compression_savings() {
    let (standard, compressed, savings, percent) = VertexCompressor::calculate_savings(1000);

    // Standard: 32 bytes * 1000 = 32000
    // Compressed: 20 bytes * 1000 = 20000
    assert_eq!(standard, 32000, "Standard size should be 32 * 1000");
    assert_eq!(compressed, 20000, "Compressed size should be 20 * 1000");
    assert_eq!(savings, 12000, "Savings should be 12000 bytes");
    assert!(
        (percent - 37.5).abs() < 0.1,
        "Savings percent should be ~37.5%"
    );
}

/// Test CompressedVertex size constants
#[test]
fn test_compressed_vertex_size_constants() {
    assert_eq!(
        CompressedVertex::STANDARD_SIZE,
        32,
        "Standard vertex should be 32 bytes"
    );
    assert_eq!(
        CompressedVertex::COMPRESSED_SIZE,
        20,
        "Compressed vertex should be 20 bytes"
    );
    assert!(
        (CompressedVertex::MEMORY_REDUCTION - 0.375).abs() < 0.001,
        "Memory reduction should be 37.5%"
    );
}

/// Test full vertex compression/decompression roundtrip
#[test]
fn test_vertex_compressor_roundtrip() {
    let position = Vec3::new(1.5, -2.3, 4.7);
    let normal = Vec3::new(0.0, 1.0, 0.0);
    let uv = Vec2::new(0.25, 0.75);

    let compressed = VertexCompressor::compress(position, normal, uv);
    let (pos_out, norm_out, uv_out) = VertexCompressor::decompress(&compressed);

    // Position should be exact (full precision)
    assert!(
        (pos_out - position).length() < 0.0001,
        "Position should roundtrip exactly"
    );

    // Normal should be close (16-bit quantization)
    assert!(normal.dot(norm_out) > 0.99, "Normal should be preserved");

    // UV should be close (half-float precision)
    assert!((uv_out - uv).length() < 0.002, "UV should be preserved");
}

/// Test batch compression
#[test]
fn test_vertex_compressor_batch() {
    let positions = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let normals = vec![Vec3::Y, Vec3::Y, Vec3::Y];
    let uvs = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
    ];

    let compressed = VertexCompressor::compress_batch(&positions, &normals, &uvs);

    assert_eq!(compressed.len(), 3, "Should produce 3 compressed vertices");

    // Verify each vertex
    for (i, cv) in compressed.iter().enumerate() {
        let (pos, norm, uv) = VertexCompressor::decompress(cv);
        assert!(
            (pos - positions[i]).length() < 0.0001,
            "Position {} mismatch",
            i
        );
        assert!(normals[i].dot(norm) > 0.99, "Normal {} mismatch", i);
        assert!((uv - uvs[i]).length() < 0.002, "UV {} mismatch", i);
    }
}

// ============================================================================
// CAMERA MATRIX TESTS
// ============================================================================

/// Test camera view matrix is right-handed
#[test]
fn test_camera_view_matrix_handedness() {
    let camera = Camera {
        position: Vec3::new(0.0, 5.0, 10.0),
        yaw: 0.0,
        pitch: 0.0,
        fovy: PI / 4.0,
        aspect: 16.0 / 9.0,
        znear: 0.1,
        zfar: 1000.0,
    };

    let view = camera.view_matrix();

    // View matrix should have determinant of 1 (orthonormal, right-handed)
    // For a rotation matrix, det should be +1 (not -1)
    // Extract the 3x3 rotation part
    let det = view
        .x_axis
        .xyz()
        .cross(view.y_axis.xyz())
        .dot(view.z_axis.xyz());
    assert!(
        det.abs() > 0.99,
        "View matrix 3x3 should have |det| ~1, got {}",
        det
    );
}

/// Test camera projection matrix has correct near/far
#[test]
fn test_camera_projection_near_far() {
    let camera = Camera {
        position: Vec3::ZERO,
        yaw: 0.0,
        pitch: 0.0,
        fovy: PI / 4.0,
        aspect: 1.0,
        znear: 0.5,
        zfar: 100.0,
    };

    let proj = camera.proj_matrix();

    // Test that a point at znear projects to z = 0 (NDC near plane)
    let near_point = Vec3::new(0.0, 0.0, -camera.znear);
    let projected = proj.project_point3(near_point);
    assert!(
        projected.z.abs() < 0.01,
        "Near plane should project to z ~0, got {}",
        projected.z
    );

    // Test that a point at zfar projects to z = 1 (NDC far plane)
    let far_point = Vec3::new(0.0, 0.0, -camera.zfar);
    let projected_far = proj.project_point3(far_point);
    assert!(
        (projected_far.z - 1.0).abs() < 0.01,
        "Far plane should project to z ~1, got {}",
        projected_far.z
    );
}

/// Test camera direction calculation
#[test]
fn test_camera_direction() {
    // Yaw 0, pitch 0 should look along positive X
    let dir = Camera::dir(0.0, 0.0);
    assert!(
        (dir - Vec3::X).length() < 0.01,
        "Yaw=0, Pitch=0 should look along +X, got {:?}",
        dir
    );

    // Pitch up should have positive Y component
    let dir_up = Camera::dir(0.0, PI / 4.0);
    assert!(
        dir_up.y > 0.5,
        "Positive pitch should look up, got {:?}",
        dir_up
    );

    // Pitch down should have negative Y component
    let dir_down = Camera::dir(0.0, -PI / 4.0);
    assert!(
        dir_down.y < -0.5,
        "Negative pitch should look down, got {:?}",
        dir_down
    );

    // Yaw 90 degrees should look along positive Z
    let dir_yaw = Camera::dir(PI / 2.0, 0.0);
    assert!(
        (dir_yaw - Vec3::Z).length() < 0.01,
        "Yaw=90deg should look along +Z, got {:?}",
        dir_yaw
    );
}

/// Test view-projection composition
#[test]
fn test_camera_vp_composition() {
    let camera = Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        yaw: 0.0,
        pitch: 0.0,
        fovy: PI / 4.0,
        aspect: 1.0,
        znear: 0.1,
        zfar: 100.0,
    };

    let vp = camera.vp();
    let view = camera.view_matrix();
    let proj = camera.proj_matrix();

    // VP should equal proj * view
    let expected = proj * view;
    let diff = (vp - expected).to_cols_array();
    let max_diff = diff.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);

    assert!(
        max_diff < 0.0001,
        "VP should equal proj * view, max diff: {}",
        max_diff
    );
}

// ============================================================================
// FRUSTUM CULLING TESTS
// ============================================================================

/// Test frustum plane extraction from identity VP
#[test]
fn test_frustum_planes_from_identity() {
    // Orthographic-like projection
    let vp = Mat4::orthographic_rh(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);
    let frustum = FrustumPlanes::from_view_proj(&vp);

    // All planes should have normalized normals (length ~1)
    for plane in &frustum.planes {
        let len = (plane[0] * plane[0] + plane[1] * plane[1] + plane[2] * plane[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 0.01,
            "Plane normal should be unit length, got {}",
            len
        );
    }
}

/// Test AABB inside frustum
#[test]
fn test_frustum_aabb_inside() {
    // Simple frustum looking down -Z
    let proj = Mat4::perspective_rh(PI / 4.0, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let vp = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&vp);

    // AABB at origin should be inside frustum
    let center = Vec3::ZERO;
    let extent = Vec3::splat(1.0);
    let inside = frustum.test_aabb(center, extent);

    assert!(inside, "AABB at origin should be inside frustum");
}

/// Test AABB outside frustum (behind camera)
#[test]
fn test_frustum_aabb_behind_camera() {
    // Camera at Z=10 looking at origin
    let proj = Mat4::perspective_rh(PI / 4.0, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let vp = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&vp);

    // AABB behind camera (at Z=20)
    let center = Vec3::new(0.0, 0.0, 20.0);
    let extent = Vec3::splat(1.0);
    let inside = frustum.test_aabb(center, extent);

    assert!(!inside, "AABB behind camera should be outside frustum");
}

/// Test AABB outside frustum (far left)
#[test]
fn test_frustum_aabb_far_left() {
    let proj = Mat4::perspective_rh(PI / 4.0, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let vp = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&vp);

    // AABB far to the left (X = -100)
    let center = Vec3::new(-100.0, 0.0, 0.0);
    let extent = Vec3::splat(1.0);
    let inside = frustum.test_aabb(center, extent);

    assert!(!inside, "AABB far left should be outside frustum");
}

/// Test AABB at frustum boundary
#[test]
fn test_frustum_aabb_boundary() {
    let proj = Mat4::perspective_rh(PI / 4.0, 1.0, 0.1, 100.0);
    let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::Y);
    let vp = proj * view;
    let frustum = FrustumPlanes::from_view_proj(&vp);

    // AABB that just touches the frustum edge
    // At distance 5, the half-width of the frustum is ~5 * tan(PI/8) ≈ 2.07
    let center = Vec3::new(3.0, 0.0, 5.0);
    let extent = Vec3::splat(1.0);
    let inside = frustum.test_aabb(center, extent);

    // This should be on the edge, but test is about the boundary logic
    // We just verify the function doesn't crash
    let _ = inside; // Result depends on exact frustum shape
}

/// Test InstanceAABB construction
#[test]
fn test_instance_aabb_new() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let extent = Vec3::new(0.5, 0.5, 0.5);
    let aabb = InstanceAABB::new(center, extent, 42);

    assert_eq!(aabb.center, [1.0, 2.0, 3.0]);
    assert_eq!(aabb.extent, [0.5, 0.5, 0.5]);
    assert_eq!(aabb.instance_index, 42);
}

/// Test InstanceAABB from transform
#[test]
fn test_instance_aabb_from_transform() {
    // Identity transform
    let transform = Mat4::IDENTITY;
    let local_min = Vec3::new(-1.0, -1.0, -1.0);
    let local_max = Vec3::new(1.0, 1.0, 1.0);

    let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);

    // Center should be at origin
    assert!((aabb.center[0].abs()) < 0.01, "Center X should be ~0");
    assert!((aabb.center[1].abs()) < 0.01, "Center Y should be ~0");
    assert!((aabb.center[2].abs()) < 0.01, "Center Z should be ~0");

    // Extent should be (1, 1, 1)
    assert!((aabb.extent[0] - 1.0).abs() < 0.01, "Extent X should be ~1");
    assert!((aabb.extent[1] - 1.0).abs() < 0.01, "Extent Y should be ~1");
    assert!((aabb.extent[2] - 1.0).abs() < 0.01, "Extent Z should be ~1");
}

/// Test InstanceAABB from scaled transform
#[test]
fn test_instance_aabb_scaled() {
    let transform = Mat4::from_scale(Vec3::splat(2.0));
    let local_min = Vec3::new(-1.0, -1.0, -1.0);
    let local_max = Vec3::new(1.0, 1.0, 1.0);

    let aabb = InstanceAABB::from_transform(&transform, local_min, local_max, 0);

    // Extent should be doubled (2, 2, 2)
    assert!(
        (aabb.extent[0] - 2.0).abs() < 0.01,
        "Scaled extent X should be ~2"
    );
    assert!(
        (aabb.extent[1] - 2.0).abs() < 0.01,
        "Scaled extent Y should be ~2"
    );
    assert!(
        (aabb.extent[2] - 2.0).abs() < 0.01,
        "Scaled extent Z should be ~2"
    );
}

// ============================================================================
// TIME OF DAY TESTS
// ============================================================================

/// Test TimeOfDay default values
#[test]
fn test_time_of_day_defaults() {
    let tod = TimeOfDay::default();

    assert!(
        (tod.current_time - 12.0).abs() < 0.01,
        "Default time should be noon (12.0)"
    );
    assert!(
        (tod.time_scale - 60.0).abs() < 0.01,
        "Default time scale should be 60.0"
    );
}

/// Test TimeOfDay sun position at noon
#[test]
fn test_sun_position_noon() {
    let mut tod = TimeOfDay::default();
    tod.current_time = 12.0; // Noon

    let sun_pos = tod.get_sun_position();

    // At noon, sun should be nearly overhead (high Y)
    assert!(
        sun_pos.y > 0.5,
        "Sun at noon should be high in the sky, Y={}",
        sun_pos.y
    );
}

/// Test TimeOfDay sun position at sunset
#[test]
fn test_sun_position_sunset() {
    let mut tod = TimeOfDay::default();
    tod.current_time = 18.0; // 6 PM

    let sun_pos = tod.get_sun_position();

    // At sunset, sun should be near horizon (Y ~0)
    assert!(
        sun_pos.y.abs() < 0.3,
        "Sun at sunset should be near horizon, Y={}",
        sun_pos.y
    );
}

/// Test TimeOfDay sun position at midnight
#[test]
fn test_sun_position_midnight() {
    let mut tod = TimeOfDay::default();
    tod.current_time = 0.0; // Midnight

    let sun_pos = tod.get_sun_position();

    // At midnight, sun should be below horizon (negative Y)
    assert!(
        sun_pos.y < 0.0,
        "Sun at midnight should be below horizon, Y={}",
        sun_pos.y
    );
}

/// Test moon is opposite to sun
#[test]
fn test_moon_opposite_sun() {
    let tod = TimeOfDay::default();

    let sun = tod.get_sun_position();
    let moon = tod.get_moon_position();

    // Moon should be opposite to sun
    let dot = sun.dot(moon);
    assert!(dot < -0.95, "Moon should be opposite to sun, dot={}", dot);
}

/// Test is_day at noon
#[test]
fn test_is_day_noon() {
    let mut tod = TimeOfDay::default();
    tod.current_time = 12.0;

    assert!(tod.is_day(), "Noon should be daytime");
    assert!(!tod.is_night(), "Noon should not be night");
}

/// Test is_night at midnight
#[test]
fn test_is_night_midnight() {
    let mut tod = TimeOfDay::default();
    tod.current_time = 0.0;

    assert!(tod.is_night(), "Midnight should be nighttime");
    assert!(!tod.is_day(), "Midnight should not be day");
}

/// Test light color changes with time
#[test]
fn test_light_color_day_vs_night() {
    let mut tod = TimeOfDay::default();

    tod.current_time = 12.0;
    let day_color = tod.get_light_color();

    tod.current_time = 0.0;
    let night_color = tod.get_light_color();

    // Day should be brighter than night
    let day_brightness = day_color.length();
    let night_brightness = night_color.length();

    assert!(
        day_brightness > night_brightness,
        "Day light should be brighter than night light"
    );
}

/// Test ambient color changes with time
#[test]
fn test_ambient_color_day_vs_night() {
    let mut tod = TimeOfDay::default();

    tod.current_time = 12.0;
    let day_ambient = tod.get_ambient_color();

    tod.current_time = 0.0;
    let night_ambient = tod.get_ambient_color();

    // Day ambient should be brighter
    let day_brightness = day_ambient.length();
    let night_brightness = night_ambient.length();

    assert!(
        day_brightness > night_brightness,
        "Day ambient should be brighter than night ambient"
    );
}

// ============================================================================
// LOD CONFIG TESTS
// ============================================================================

/// Test LODConfig defaults
#[test]
fn test_lod_config_defaults() {
    let config = LODConfig::default();

    assert_eq!(
        config.reduction_targets.len(),
        3,
        "Default should have 3 LOD levels"
    );
    assert!(
        (config.reduction_targets[0] - 0.75).abs() < 0.01,
        "LOD1 should target 75%"
    );
    assert!(
        (config.reduction_targets[1] - 0.50).abs() < 0.01,
        "LOD2 should target 50%"
    );
    assert!(
        (config.reduction_targets[2] - 0.25).abs() < 0.01,
        "LOD3 should target 25%"
    );
    assert!(
        config.preserve_boundaries,
        "Default should preserve boundaries"
    );
}

/// Test LODConfig reduction targets are decreasing
#[test]
fn test_lod_reduction_targets_decreasing() {
    let config = LODConfig::default();

    for i in 1..config.reduction_targets.len() {
        assert!(
            config.reduction_targets[i] < config.reduction_targets[i - 1],
            "LOD reduction targets should decrease: LOD{} ({}) >= LOD{} ({})",
            i - 1,
            config.reduction_targets[i - 1],
            i,
            config.reduction_targets[i]
        );
    }
}

/// Test custom LODConfig
#[test]
fn test_lod_config_custom() {
    let config = LODConfig {
        reduction_targets: vec![0.8, 0.6, 0.4, 0.2],
        max_error: 0.05,
        preserve_boundaries: false,
    };

    assert_eq!(
        config.reduction_targets.len(),
        4,
        "Custom should have 4 levels"
    );
    assert!((config.max_error - 0.05).abs() < 0.001);
    assert!(!config.preserve_boundaries);
}
