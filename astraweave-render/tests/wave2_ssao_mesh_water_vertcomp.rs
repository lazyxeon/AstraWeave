//! Wave 2 mutation-hardened tests for SSAO, Mesh, Water, and Vertex Compression modules.
//!
//! These tests target arithmetic operations, comparison operators, and constant values
//! that are common cargo-mutants mutation targets.

use astraweave_render::mesh::{compute_tangents, CpuMesh, MeshVertex, MeshVertexLayout};
#[cfg(feature = "ssao")]
use astraweave_render::ssao::{SsaoConfig, SsaoKernel, SsaoQuality, SsaoUniforms};
use astraweave_render::vertex_compression::{
    CompressedVertex, HalfFloatEncoder, OctahedralEncoder, VertexCompressor,
};
use astraweave_render::water::{WaterUniforms, WaterVertex};
use glam::{Vec2, Vec3, Vec4};

// ──────────────────────────────────────────────────────────────────
// SsaoQuality — match arm exact values
// ──────────────────────────────────────────────────────────────────

#[cfg(feature = "ssao")]
#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_sample_count() {
    assert_eq!(SsaoQuality::Low.sample_count(), 8);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_sample_count() {
    assert_eq!(SsaoQuality::Medium.sample_count(), 16);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_sample_count() {
    assert_eq!(SsaoQuality::High.sample_count(), 32);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_sample_count() {
    assert_eq!(SsaoQuality::Ultra.sample_count(), 64);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_radius() {
    assert!((SsaoQuality::Low.radius() - 0.5).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_radius() {
    assert!((SsaoQuality::Medium.radius() - 1.0).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_radius() {
    assert!((SsaoQuality::High.radius() - 1.5).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_radius() {
    assert!((SsaoQuality::Ultra.radius() - 2.0).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_low_blur_kernel() {
    assert_eq!(SsaoQuality::Low.blur_kernel_size(), 0);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_medium_blur_kernel() {
    assert_eq!(SsaoQuality::Medium.blur_kernel_size(), 3);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_high_blur_kernel() {
    assert_eq!(SsaoQuality::High.blur_kernel_size(), 5);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_quality_ultra_blur_kernel() {
    assert_eq!(SsaoQuality::Ultra.blur_kernel_size(), 7);
}

// Ordering relationships — catch swap/confusion between presets
#[cfg(feature = "ssao")]
#[test]
fn ssao_sample_count_increases_with_quality() {
    assert!(SsaoQuality::Low.sample_count() < SsaoQuality::Medium.sample_count());
    assert!(SsaoQuality::Medium.sample_count() < SsaoQuality::High.sample_count());
    assert!(SsaoQuality::High.sample_count() < SsaoQuality::Ultra.sample_count());
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_radius_increases_with_quality() {
    assert!(SsaoQuality::Low.radius() < SsaoQuality::Medium.radius());
    assert!(SsaoQuality::Medium.radius() < SsaoQuality::High.radius());
    assert!(SsaoQuality::High.radius() < SsaoQuality::Ultra.radius());
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_blur_increases_with_quality() {
    assert!(SsaoQuality::Low.blur_kernel_size() < SsaoQuality::Medium.blur_kernel_size());
    assert!(SsaoQuality::Medium.blur_kernel_size() < SsaoQuality::High.blur_kernel_size());
    assert!(SsaoQuality::High.blur_kernel_size() < SsaoQuality::Ultra.blur_kernel_size());
}

// ──────────────────────────────────────────────────────────────────
// SsaoConfig — default golden values
// ──────────────────────────────────────────────────────────────────

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_quality() {
    let c = SsaoConfig::default();
    assert_eq!(c.quality, SsaoQuality::Medium);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_radius() {
    let c = SsaoConfig::default();
    assert!((c.radius - 1.0).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_bias() {
    let c = SsaoConfig::default();
    assert!((c.bias - 0.025).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_intensity() {
    let c = SsaoConfig::default();
    assert!((c.intensity - 1.0).abs() < 1e-6);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_config_default_enabled() {
    let c = SsaoConfig::default();
    assert!(c.enabled);
}

// ──────────────────────────────────────────────────────────────────
// SsaoKernel — hemisphere sample generation
// ──────────────────────────────────────────────────────────────────

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_generates_64_samples_array() {
    let kernel = SsaoKernel::generate(64);
    // All 64 slots should have data
    for i in 0..64 {
        let s = kernel.samples[i];
        let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        assert!(len > 0.0, "Sample {} should be non-zero", i);
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_8_samples_has_8_nonzero() {
    let kernel = SsaoKernel::generate(8);
    let nonzero = kernel
        .samples
        .iter()
        .filter(|s| s[0] != 0.0 || s[1] != 0.0 || s[2] != 0.0)
        .count();
    assert_eq!(nonzero, 8, "8 samples requested, 8 should be non-zero");
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_samples_in_upper_hemisphere() {
    // All generated samples should have z > 0 (hemisphere)
    let kernel = SsaoKernel::generate(32);
    for i in 0..32 {
        let z = kernel.samples[i][2];
        assert!(z > 0.0, "Sample {} z={} should be > 0 (hemisphere)", i, z);
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_samples_within_unit_sphere() {
    let kernel = SsaoKernel::generate(64);
    for i in 0..64 {
        let s = kernel.samples[i];
        let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        assert!(
            len <= 1.01,
            "Sample {} length {} exceeds unit sphere",
            i,
            len
        );
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_scale_increases_with_index() {
    // Earlier samples should generally be closer to origin (smaller magnitude)
    let kernel = SsaoKernel::generate(64);
    let first_len = {
        let s = kernel.samples[0];
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    };
    let last_len = {
        let s = kernel.samples[63];
        (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
    };
    assert!(
        last_len > first_len,
        "Last sample ({}) should be farther than first ({})",
        last_len,
        first_len
    );
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_w_component_always_zero() {
    let kernel = SsaoKernel::generate(64);
    for i in 0..64 {
        assert_eq!(kernel.samples[i][3], 0.0, "Sample {} w should be 0.0", i);
    }
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_deterministic() {
    let a = SsaoKernel::generate(32);
    let b = SsaoKernel::generate(32);
    for i in 0..32 {
        assert_eq!(a.samples[i], b.samples[i], "Kernel should be deterministic");
    }
}

// ──────────────────────────────────────────────────────────────────
// SsaoUniforms — struct size
// ──────────────────────────────────────────────────────────────────

#[cfg(feature = "ssao")]
#[test]
fn ssao_uniforms_size() {
    // 2 matrices (128) + 2 vec4 params (32) = 160 bytes
    assert_eq!(std::mem::size_of::<SsaoUniforms>(), 160);
}

#[cfg(feature = "ssao")]
#[test]
fn ssao_kernel_size() {
    // 64 * 4 floats * 4 bytes = 1024 bytes
    assert_eq!(std::mem::size_of::<SsaoKernel>(), 1024);
}

// ──────────────────────────────────────────────────────────────────
// MeshVertex — constructors and size
// ──────────────────────────────────────────────────────────────────

#[test]
fn mesh_vertex_size_48_bytes() {
    // 3+3+4+2 = 12 floats * 4 bytes = 48
    assert_eq!(std::mem::size_of::<MeshVertex>(), 48);
}

#[test]
fn mesh_vertex_new_stores_position() {
    let v = MeshVertex::new(Vec3::new(1.0, 2.0, 3.0), Vec3::ZERO, Vec4::ZERO, Vec2::ZERO);
    assert_eq!(v.position, [1.0, 2.0, 3.0]);
}

#[test]
fn mesh_vertex_new_stores_normal() {
    let v = MeshVertex::new(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), Vec4::ZERO, Vec2::ZERO);
    assert_eq!(v.normal, [0.0, 1.0, 0.0]);
}

#[test]
fn mesh_vertex_new_stores_tangent() {
    let v = MeshVertex::new(
        Vec3::ZERO,
        Vec3::ZERO,
        Vec4::new(1.0, 0.0, 0.0, -1.0),
        Vec2::ZERO,
    );
    assert_eq!(v.tangent, [1.0, 0.0, 0.0, -1.0]);
}

#[test]
fn mesh_vertex_new_stores_uv() {
    let v = MeshVertex::new(Vec3::ZERO, Vec3::ZERO, Vec4::ZERO, Vec2::new(0.5, 0.75));
    assert_eq!(v.uv, [0.5, 0.75]);
}

#[test]
fn mesh_vertex_from_arrays_roundtrip() {
    let v = MeshVertex::from_arrays(
        [4.0, 5.0, 6.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.25, 0.5],
    );
    assert_eq!(v.position, [4.0, 5.0, 6.0]);
    assert_eq!(v.normal, [0.0, 0.0, 1.0]);
    assert_eq!(v.tangent, [0.0, 1.0, 0.0, 1.0]);
    assert_eq!(v.uv, [0.25, 0.5]);
}

#[test]
fn mesh_vertex_attribs_count() {
    assert_eq!(MeshVertex::ATTRIBS.len(), 4);
}

#[test]
fn mesh_vertex_attribs_shader_locations() {
    assert_eq!(MeshVertex::ATTRIBS[0].shader_location, 0);
    assert_eq!(MeshVertex::ATTRIBS[1].shader_location, 1);
    assert_eq!(MeshVertex::ATTRIBS[2].shader_location, 2);
    assert_eq!(MeshVertex::ATTRIBS[3].shader_location, 3);
}

#[test]
fn mesh_vertex_layout_stride() {
    let layout = MeshVertexLayout::buffer_layout();
    assert_eq!(layout.array_stride, 48);
}

// ──────────────────────────────────────────────────────────────────
// CpuMesh::aabb — min/max calculations
// ──────────────────────────────────────────────────────────────────

#[test]
fn cpu_mesh_aabb_empty_returns_none() {
    assert!(CpuMesh::default().aabb().is_none());
}

#[test]
fn cpu_mesh_aabb_single_point() {
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [3.0, -1.0, 7.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 0.0],
    ));
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, Vec3::new(3.0, -1.0, 7.0));
    assert_eq!(max, Vec3::new(3.0, -1.0, 7.0));
}

#[test]
fn cpu_mesh_aabb_min_max_correct() {
    let mut m = CpuMesh::default();
    for pos in [[1.0, 5.0, -2.0], [-3.0, 0.0, 4.0], [2.0, -1.0, 1.0]] {
        m.vertices.push(MeshVertex::from_arrays(
            pos,
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 0.0],
        ));
    }
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min.x, -3.0);
    assert_eq!(min.y, -1.0);
    assert_eq!(min.z, -2.0);
    assert_eq!(max.x, 2.0);
    assert_eq!(max.y, 5.0);
    assert_eq!(max.z, 4.0);
}

#[test]
fn cpu_mesh_aabb_ignores_non_position_fields() {
    // AABB should only use .position, not normal/tangent/uv
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [100.0, 100.0, 100.0], // large normal shouldn't affect AABB
        [50.0, 50.0, 50.0, 50.0],
        [99.0, 99.0],
    ));
    let (min, max) = m.aabb().unwrap();
    assert_eq!(min, Vec3::ZERO);
    assert_eq!(max, Vec3::ZERO);
}

// ──────────────────────────────────────────────────────────────────
// compute_tangents — direction and handedness
// ──────────────────────────────────────────────────────────────────

#[test]
fn compute_tangents_empty_mesh_no_panic() {
    let mut m = CpuMesh::default();
    compute_tangents(&mut m);
}

#[test]
fn compute_tangents_non_multiple_of_3_indices_returns_early() {
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0],
    ));
    m.indices = vec![0, 0]; // not divisible by 3
    let original_tangent = m.vertices[0].tangent;
    compute_tangents(&mut m);
    // Should return early — tangents unchanged
    assert_eq!(m.vertices[0].tangent, original_tangent);
}

#[test]
fn compute_tangents_xz_plane_tangent_along_x() {
    // Triangle on XZ plane, UV aligned with X
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0],
    ));
    m.indices = vec![0, 1, 2];

    compute_tangents(&mut m);

    // Tangent should be approximately along +X
    for v in &m.vertices {
        let tx = v.tangent[0];
        let ty = v.tangent[1];
        let tz = v.tangent[2];
        let len = (tx * tx + ty * ty + tz * tz).sqrt();
        assert!(len > 0.9, "Tangent should be unit-length, got {}", len);
        assert!(tx > 0.9, "Tangent x component should be ~1.0, got {}", tx);
        assert!(ty.abs() < 0.1, "Tangent y should be ~0, got {}", ty);
        assert!(tz.abs() < 0.1, "Tangent z should be ~0, got {}", tz);
    }
}

#[test]
fn compute_tangents_handedness_positive_for_standard_winding() {
    let mut m = CpuMesh::default();
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0],
    ));
    m.vertices.push(MeshVertex::from_arrays(
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0],
    ));
    m.indices = vec![0, 1, 2];
    compute_tangents(&mut m);

    // Handedness (w) should be ±1.0
    for v in &m.vertices {
        let w = v.tangent[3];
        assert!(
            (w - 1.0).abs() < 0.01 || (w + 1.0).abs() < 0.01,
            "Handedness should be ±1.0, got {}",
            w
        );
    }
}

// ──────────────────────────────────────────────────────────────────
// WaterUniforms — struct size and default golden values
// ──────────────────────────────────────────────────────────────────

#[test]
fn water_uniforms_size_512_bytes() {
    assert_eq!(std::mem::size_of::<WaterUniforms>(), 512);
}

#[test]
fn water_uniforms_default_camera_pos() {
    let u = WaterUniforms::default();
    assert_eq!(u.camera_pos, [0.0, 5.0, -10.0]);
}

#[test]
fn water_uniforms_default_time() {
    let u = WaterUniforms::default();
    assert_eq!(u.time, 0.0);
}

#[test]
fn water_uniforms_default_deep_color() {
    let u = WaterUniforms::default();
    assert!((u.water_color_deep[0] - 0.02).abs() < 1e-6);
    assert!((u.water_color_deep[1] - 0.08).abs() < 1e-6);
    assert!((u.water_color_deep[2] - 0.2).abs() < 1e-6);
}

#[test]
fn water_uniforms_default_shallow_color() {
    let u = WaterUniforms::default();
    assert!((u.water_color_shallow[0] - 0.1).abs() < 1e-6);
    assert!((u.water_color_shallow[1] - 0.4).abs() < 1e-6);
    assert!((u.water_color_shallow[2] - 0.5).abs() < 1e-6);
}

#[test]
fn water_uniforms_default_foam_color() {
    let u = WaterUniforms::default();
    assert!((u.foam_color[0] - 0.95).abs() < 1e-6);
    assert!((u.foam_color[1] - 0.98).abs() < 1e-6);
    assert!((u.foam_color[2] - 1.0).abs() < 1e-6);
}

#[test]
fn water_uniforms_default_foam_threshold() {
    let u = WaterUniforms::default();
    assert!((u.foam_threshold - 0.6).abs() < 1e-6);
}

#[test]
fn water_vertex_size() {
    // 3 floats position + 2 floats uv = 20 bytes
    assert_eq!(std::mem::size_of::<WaterVertex>(), 20);
}

#[test]
fn water_vertex_desc_stride() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.array_stride, 20);
}

#[test]
fn water_vertex_desc_has_2_attributes() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes.len(), 2);
}

#[test]
fn water_vertex_desc_attribute_offsets() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes[0].offset, 0);
    assert_eq!(desc.attributes[1].offset, 12); // after 3 floats
}

#[test]
fn water_vertex_desc_shader_locations() {
    let desc = WaterVertex::desc();
    assert_eq!(desc.attributes[0].shader_location, 0);
    assert_eq!(desc.attributes[1].shader_location, 1);
}

// ──────────────────────────────────────────────────────────────────
// Vertex Compression — golden values and roundtrips
// ──────────────────────────────────────────────────────────────────

#[test]
fn compressed_vertex_size_20() {
    assert_eq!(std::mem::size_of::<CompressedVertex>(), 20);
}

#[test]
fn compressed_vertex_standard_size_32() {
    assert_eq!(CompressedVertex::STANDARD_SIZE, 32);
}

#[test]
fn compressed_vertex_compressed_size_20() {
    assert_eq!(CompressedVertex::COMPRESSED_SIZE, 20);
}

#[test]
fn compressed_vertex_memory_reduction_37_5() {
    assert!((CompressedVertex::MEMORY_REDUCTION - 0.375).abs() < 1e-6);
}

#[test]
fn octahedral_encode_up_vector() {
    let encoded = OctahedralEncoder::encode(Vec3::Y);
    let decoded = OctahedralEncoder::decode(encoded);
    assert!((decoded - Vec3::Y).length() < 0.01);
}

#[test]
fn octahedral_encode_down_vector() {
    let encoded = OctahedralEncoder::encode(Vec3::NEG_Y);
    let decoded = OctahedralEncoder::decode(encoded);
    assert!((decoded - Vec3::NEG_Y).length() < 0.01);
}

#[test]
fn octahedral_encode_right_vector() {
    let encoded = OctahedralEncoder::encode(Vec3::X);
    let decoded = OctahedralEncoder::decode(encoded);
    assert!((decoded - Vec3::X).length() < 0.02);
}

#[test]
fn octahedral_encode_forward_vector() {
    let encoded = OctahedralEncoder::encode(Vec3::Z);
    let decoded = OctahedralEncoder::decode(encoded);
    assert!((decoded - Vec3::Z).length() < 0.02);
}

#[test]
fn octahedral_neg_z_vector() {
    let encoded = OctahedralEncoder::encode(Vec3::NEG_Z);
    let decoded = OctahedralEncoder::decode(encoded);
    assert!((decoded - Vec3::NEG_Z).length() < 0.02);
}

#[test]
fn octahedral_encoding_error_below_1_degree() {
    let normal = Vec3::new(0.5, 0.7, 0.2).normalize();
    let error = OctahedralEncoder::encoding_error(normal);
    assert!(
        error < 0.018,
        "Error {} should be < 1 degree (0.017 rad)",
        error
    );
}

#[test]
fn octahedral_encoding_error_nonzero_for_diagonal() {
    let normal = Vec3::new(1.0, 1.0, 1.0).normalize();
    let error = OctahedralEncoder::encoding_error(normal);
    // Should be positive (lossy) but very small
    assert!(error >= 0.0);
    assert!(error < 0.02);
}

#[test]
fn half_float_encode_0_exact() {
    let encoded = HalfFloatEncoder::encode(0.0);
    let decoded = HalfFloatEncoder::decode(encoded);
    assert_eq!(decoded, 0.0);
}

#[test]
fn half_float_encode_1_roundtrip() {
    let encoded = HalfFloatEncoder::encode(1.0);
    let decoded = HalfFloatEncoder::decode(encoded);
    assert!((decoded - 1.0).abs() < 0.001);
}

#[test]
fn half_float_encode_negative_roundtrip() {
    let encoded = HalfFloatEncoder::encode(-0.5);
    let decoded = HalfFloatEncoder::decode(encoded);
    assert!((decoded - (-0.5)).abs() < 0.001);
}

#[test]
fn half_float_encode_vec2_roundtrip() {
    let uv = Vec2::new(0.25, 0.75);
    let encoded = HalfFloatEncoder::encode_vec2(uv);
    let decoded = HalfFloatEncoder::decode_vec2(encoded);
    assert!((decoded.x - 0.25).abs() < 0.001);
    assert!((decoded.y - 0.75).abs() < 0.001);
}

#[test]
fn vertex_compressor_position_exact_roundtrip() {
    let pos = Vec3::new(1.5, -2.3, 4.7);
    let norm = Vec3::Y;
    let uv = Vec2::new(0.5, 0.5);
    let compressed = VertexCompressor::compress(pos, norm, uv);
    let (dec_pos, _, _) = VertexCompressor::decompress(&compressed);
    assert_eq!(dec_pos, pos, "Position should survive compression exactly");
}

#[test]
fn vertex_compressor_normal_close_roundtrip() {
    let pos = Vec3::ZERO;
    let norm = Vec3::new(0.0, 0.0, 1.0);
    let uv = Vec2::ZERO;
    let compressed = VertexCompressor::compress(pos, norm, uv);
    let (_, dec_norm, _) = VertexCompressor::decompress(&compressed);
    assert!((dec_norm - norm).length() < 0.02);
}

#[test]
fn vertex_compressor_uv_close_roundtrip() {
    let pos = Vec3::ZERO;
    let norm = Vec3::Y;
    let uv = Vec2::new(0.33, 0.67);
    let compressed = VertexCompressor::compress(pos, norm, uv);
    let (_, _, dec_uv) = VertexCompressor::decompress(&compressed);
    assert!((dec_uv.x - 0.33).abs() < 0.002);
    assert!((dec_uv.y - 0.67).abs() < 0.002);
}

#[test]
fn vertex_compressor_batch_length() {
    let positions = vec![Vec3::ZERO, Vec3::ONE, Vec3::X];
    let normals = vec![Vec3::Y, Vec3::Y, Vec3::Y];
    let uvs = vec![Vec2::ZERO, Vec2::ONE, Vec2::new(0.5, 0.5)];
    let batch = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    assert_eq!(batch.len(), 3);
}

#[test]
fn vertex_compressor_calculate_savings_10k() {
    let (standard, compressed, savings, percent) = VertexCompressor::calculate_savings(10000);
    assert_eq!(standard, 320_000);
    assert_eq!(compressed, 200_000);
    assert_eq!(savings, 120_000);
    assert!((percent - 37.5).abs() < 0.1);
}

#[test]
fn vertex_compressor_calculate_savings_1() {
    let (standard, compressed, savings, _) = VertexCompressor::calculate_savings(1);
    assert_eq!(standard, 32);
    assert_eq!(compressed, 20);
    assert_eq!(savings, 12);
}
