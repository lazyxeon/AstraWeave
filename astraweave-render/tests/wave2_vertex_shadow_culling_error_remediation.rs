#![allow(unused, clippy::all)]
//! Batch 8: Vertex compression, shadow CSM, nanite GPU culling, megalights, errors
//! Mutation-resistant integration tests targeting:
//!   - OctahedralEncoder: encode/decode roundtrip, axis vectors, error bounds
//!   - HalfFloatEncoder: encode/decode roundtrip, Vec2, edge values
//!   - VertexCompressor: compress/decompress, batch, savings calculation
//!   - CompressedVertex: constants (STANDARD_SIZE, COMPRESSED_SIZE, MEMORY_REDUCTION)
//!   - shadow_csm: CASCADE_COUNT, CASCADE_RESOLUTION, ATLAS_RESOLUTION, DEPTH_BIAS
//!   - GpuShadowCascade: size, Pod/Zeroable, From<&ShadowCascade>
//!   - CullStats: default all zeros, size 32 bytes, Pod/Zeroable
//!   - GpuCamera: from_matrix populates fields correctly
//!   - ClusterBounds: size 32 bytes, Pod/Zeroable
//!   - GpuLight (megalights): size 32 bytes, Pod/Zeroable
//!   - RenderError: variant Display strings, From<io::Error>

use glam::{Mat4, Vec2, Vec3};

use astraweave_render::clustered_megalights::{ClusterBounds, GpuLight};
use astraweave_render::error::RenderError;
#[cfg(feature = "nanite")]
use astraweave_render::nanite_gpu_culling::{CullStats, GpuCamera};
use astraweave_render::shadow_csm::{
    GpuShadowCascade, ShadowCascade, ATLAS_RESOLUTION, CASCADE_COUNT, CASCADE_RESOLUTION,
    DEPTH_BIAS,
};
use astraweave_render::vertex_compression::{
    CompressedVertex, HalfFloatEncoder, OctahedralEncoder, VertexCompressor,
};

// ═══════════════════════════════════════════════════════════════════════════════
//  CompressedVertex constants
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn compressed_vertex_standard_size_32() {
    assert_eq!(CompressedVertex::STANDARD_SIZE, 32);
}

#[test]
fn compressed_vertex_compressed_size_20() {
    assert_eq!(CompressedVertex::COMPRESSED_SIZE, 20);
}

#[test]
fn compressed_vertex_memory_reduction_37_5_percent() {
    assert!((CompressedVertex::MEMORY_REDUCTION - 0.375).abs() < 1e-6);
}

#[test]
fn compressed_vertex_actual_size_matches_constant() {
    assert_eq!(
        std::mem::size_of::<CompressedVertex>(),
        CompressedVertex::COMPRESSED_SIZE,
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
//  OctahedralEncoder
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn octahedral_encode_up_vector() {
    let enc = OctahedralEncoder::encode(Vec3::Y);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::Y).length() < 0.01, "up vector roundtrip");
}

#[test]
fn octahedral_encode_right_vector() {
    let enc = OctahedralEncoder::encode(Vec3::X);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::X).length() < 0.01, "right vector roundtrip");
}

#[test]
fn octahedral_encode_forward_vector() {
    let enc = OctahedralEncoder::encode(Vec3::Z);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::Z).length() < 0.01, "forward vector roundtrip");
}

#[test]
fn octahedral_encode_negative_axes() {
    for dir in [Vec3::NEG_X, Vec3::NEG_Y, Vec3::NEG_Z] {
        let enc = OctahedralEncoder::encode(dir);
        let dec = OctahedralEncoder::decode(enc);
        assert!(
            (dec - dir).length() < 0.02,
            "neg axis roundtrip failed for {dir:?}"
        );
    }
}

#[test]
fn octahedral_encode_diagonal_normalized() {
    let normal = Vec3::new(1.0, 1.0, 1.0).normalize();
    let enc = OctahedralEncoder::encode(normal);
    let dec = OctahedralEncoder::decode(enc);
    let error = (dec - normal).length();
    assert!(error < 0.02, "diagonal roundtrip error {error}");
}

#[test]
fn octahedral_decode_produces_unit_vector() {
    for &n in &[
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::new(0.3, 0.5, 0.8).normalize(),
    ] {
        let enc = OctahedralEncoder::encode(n);
        let dec = OctahedralEncoder::decode(enc);
        let len = dec.length();
        assert!(
            (len - 1.0).abs() < 0.001,
            "decoded not unit: length = {len}"
        );
    }
}

#[test]
fn octahedral_encoding_error_small() {
    let normal = Vec3::new(0.5, 0.7, 0.3).normalize();
    let error_rad = OctahedralEncoder::encoding_error(normal);
    // With 16-bit quantization error should be very small (< 0.01 radians ≈ 0.57°)
    assert!(error_rad < 0.01, "error {error_rad} rad too large");
}

#[test]
fn octahedral_encoding_error_axis_aligned_near_zero() {
    let error = OctahedralEncoder::encoding_error(Vec3::Y);
    assert!(error < 0.001, "axis aligned error should be ~0");
}

#[test]
fn octahedral_lower_hemisphere() {
    // z < 0 triggers the wrapping path
    let normal = Vec3::new(0.0, 0.0, -1.0);
    let enc = OctahedralEncoder::encode(normal);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - normal).length() < 0.02, "lower hemisphere roundtrip");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HalfFloatEncoder
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn half_float_encode_zero() {
    let enc = HalfFloatEncoder::encode(0.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 0.0).abs() < 1e-6);
}

#[test]
fn half_float_encode_one() {
    let enc = HalfFloatEncoder::encode(1.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 1.0).abs() < 0.001);
}

#[test]
fn half_float_encode_half() {
    let enc = HalfFloatEncoder::encode(0.5);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 0.5).abs() < 0.001);
}

#[test]
fn half_float_encode_negative() {
    let enc = HalfFloatEncoder::encode(-1.5);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - (-1.5)).abs() < 0.002);
}

#[test]
fn half_float_encode_vec2_roundtrip() {
    let uv = Vec2::new(0.75, 0.25);
    let enc = HalfFloatEncoder::encode_vec2(uv);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert!((dec.x - 0.75).abs() < 0.001);
    assert!((dec.y - 0.25).abs() < 0.001);
}

#[test]
fn half_float_encode_vec2_zero() {
    let enc = HalfFloatEncoder::encode_vec2(Vec2::ZERO);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert!(dec.length() < 1e-5);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VertexCompressor
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn vertex_compressor_roundtrip() {
    let pos = Vec3::new(1.0, 2.0, 3.0);
    let normal = Vec3::Y;
    let uv = Vec2::new(0.5, 0.5);

    let compressed = VertexCompressor::compress(pos, normal, uv);
    let (d_pos, d_normal, d_uv) = VertexCompressor::decompress(&compressed);

    assert!((d_pos - pos).length() < 1e-6, "position exact");
    assert!((d_normal - normal).length() < 0.01, "normal close");
    assert!((d_uv - uv).length() < 0.002, "uv close");
}

#[test]
fn vertex_compressor_position_exact() {
    // Positions are stored as full f32 — no loss
    let pos = Vec3::new(123.456, -789.012, 0.001);
    let compressed = VertexCompressor::compress(pos, Vec3::Y, Vec2::ZERO);
    let (d_pos, _, _) = VertexCompressor::decompress(&compressed);
    assert_eq!(d_pos, pos);
}

#[test]
fn vertex_compressor_batch_length() {
    let positions = vec![Vec3::ZERO; 100];
    let normals = vec![Vec3::Y; 100];
    let uvs = vec![Vec2::ZERO; 100];
    let batch = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    assert_eq!(batch.len(), 100);
}

#[test]
#[should_panic(expected = "Position and normal counts must match")]
fn vertex_compressor_batch_mismatched_normals_panics() {
    let positions = vec![Vec3::ZERO; 3];
    let normals = vec![Vec3::Y; 2]; // mismatch
    let uvs = vec![Vec2::ZERO; 3];
    let _ = VertexCompressor::compress_batch(&positions, &normals, &uvs);
}

#[test]
#[should_panic(expected = "Position and UV counts must match")]
fn vertex_compressor_batch_mismatched_uvs_panics() {
    let positions = vec![Vec3::ZERO; 3];
    let normals = vec![Vec3::Y; 3];
    let uvs = vec![Vec2::ZERO; 1]; // mismatch
    let _ = VertexCompressor::compress_batch(&positions, &normals, &uvs);
}

#[test]
fn vertex_compressor_savings_1000_vertices() {
    let (standard, compressed, savings, pct) = VertexCompressor::calculate_savings(1000);
    assert_eq!(standard, 1000 * 32);
    assert_eq!(compressed, 1000 * 20);
    assert_eq!(savings, 1000 * 12);
    assert!((pct - 37.5).abs() < 0.1);
}

#[test]
fn vertex_compressor_savings_zero_vertices() {
    let (standard, compressed, savings, _) = VertexCompressor::calculate_savings(0);
    assert_eq!(standard, 0);
    assert_eq!(compressed, 0);
    assert_eq!(savings, 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Shadow CSM constants
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn cascade_count_is_4() {
    assert_eq!(CASCADE_COUNT, 4);
}

#[test]
fn cascade_resolution_2048() {
    assert_eq!(CASCADE_RESOLUTION, 2048);
}

#[test]
fn atlas_resolution_equals_cascade() {
    assert_eq!(ATLAS_RESOLUTION, CASCADE_RESOLUTION);
}

#[test]
fn depth_bias_0_005() {
    assert!((DEPTH_BIAS - 0.005).abs() < 1e-6);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuShadowCascade
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn gpu_shadow_cascade_size() {
    // view_proj: [f32;4]×4 = 64B + split_distances: [f32;4] = 16B + atlas_transform: [f32;4] = 16B = 96B
    assert_eq!(std::mem::size_of::<GpuShadowCascade>(), 96);
}

#[test]
fn gpu_shadow_cascade_zeroed() {
    let c: GpuShadowCascade = bytemuck::Zeroable::zeroed();
    assert_eq!(c.split_distances, [0.0; 4]);
    assert_eq!(c.atlas_transform, [0.0; 4]);
}

#[test]
fn gpu_shadow_cascade_from_shadow_cascade() {
    use glam::Vec4;
    let cascade = ShadowCascade {
        near: 0.1,
        far: 10.0,
        view_matrix: Mat4::IDENTITY,
        proj_matrix: Mat4::IDENTITY,
        view_proj_matrix: Mat4::IDENTITY,
        atlas_offset: Vec4::new(0.0, 0.5, 0.5, 0.5),
    };
    let gpu: GpuShadowCascade = (&cascade).into();
    assert!((gpu.split_distances[0] - 0.1).abs() < 1e-6, "near");
    assert!((gpu.split_distances[1] - 10.0).abs() < 1e-6, "far");
    assert!((gpu.split_distances[2] - 0.0).abs() < 1e-6, "pad0");
    assert!(
        (gpu.atlas_transform[1] - 0.5).abs() < 1e-6,
        "atlas offset y"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
//  CullStats (nanite GPU culling) — requires "nanite" feature
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "nanite")]
#[test]
fn cull_stats_default_all_zero() {
    let s = CullStats::default();
    assert_eq!(s.total_clusters, 0);
    assert_eq!(s.frustum_culled, 0);
    assert_eq!(s.occlusion_culled, 0);
    assert_eq!(s.backface_culled, 0);
    assert_eq!(s.visible_count, 0);
}

#[cfg(feature = "nanite")]
#[test]
fn cull_stats_size_32_bytes() {
    // 5 u32 + 3 padding u32 = 8 × 4 = 32
    assert_eq!(std::mem::size_of::<CullStats>(), 32);
}

#[cfg(feature = "nanite")]
#[test]
fn cull_stats_pod_roundtrip() {
    let mut s = CullStats::default();
    s.total_clusters = 1000;
    s.visible_count = 750;
    let bytes = bytemuck::bytes_of(&s);
    assert_eq!(bytes.len(), 32);
    let back: &CullStats = bytemuck::from_bytes(bytes);
    assert_eq!(back.total_clusters, 1000);
    assert_eq!(back.visible_count, 750);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  GpuCamera (nanite GPU culling) — requires "nanite" feature
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_from_matrix_stores_position() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::new(1.0, 2.0, 3.0), 1920, 1080);
    assert_eq!(cam.position, [1.0, 2.0, 3.0]);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_from_matrix_stores_screen_dims() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::ZERO, 1920, 1080);
    assert_eq!(cam.screen_width, 1920);
    assert_eq!(cam.screen_height, 1080);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_from_matrix_occlusion_enabled() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::ZERO, 1920, 1080);
    assert_eq!(cam.enable_occlusion, 1);
    assert_eq!(cam.enable_backface, 1);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_from_matrix_lod_scale_default_1() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::ZERO, 1920, 1080);
    assert!((cam.lod_scale - 1.0).abs() < 1e-6);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_hiz_mip_count() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::ZERO, 1024, 768);
    // max(1024, 768) = 1024 → log2(1024) = 10 → ceil(10) = 10
    assert_eq!(cam.hiz_mip_count, 10);
}

#[cfg(feature = "nanite")]
#[test]
fn gpu_camera_hiz_size_matches_screen() {
    let cam = GpuCamera::from_matrix(Mat4::IDENTITY, Vec3::ZERO, 2560, 1440);
    assert_eq!(cam.hiz_size, [2560, 1440]);
}

// ═══════════════════════════════════════════════════════════════════════════════
//  ClusterBounds + GpuLight (megalights)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn cluster_bounds_size_32_bytes() {
    // min_pos[3](12) + pad(4) + max_pos[3](12) + pad(4) = 32
    assert_eq!(std::mem::size_of::<ClusterBounds>(), 32);
}

#[test]
fn cluster_bounds_zeroed() {
    let b: ClusterBounds = bytemuck::Zeroable::zeroed();
    assert_eq!(b.min_pos, [0.0; 3]);
    assert_eq!(b.max_pos, [0.0; 3]);
}

#[test]
fn gpu_light_megalights_size_32_bytes() {
    // position[4](16) + color[4](16) = 32
    assert_eq!(std::mem::size_of::<GpuLight>(), 32);
}

#[test]
fn gpu_light_megalights_zeroed() {
    let l: GpuLight = bytemuck::Zeroable::zeroed();
    assert_eq!(l.position, [0.0; 4]);
    assert_eq!(l.color, [0.0; 4]);
}

#[test]
fn gpu_light_megalights_pod_roundtrip() {
    let l = GpuLight {
        position: [1.0, 2.0, 3.0, 5.0], // xyz=pos, w=radius
        color: [1.0, 0.8, 0.6, 100.0],  // rgb=color, a=intensity
    };
    let bytes = bytemuck::bytes_of(&l);
    assert_eq!(bytes.len(), 32);
    let back: &GpuLight = bytemuck::from_bytes(bytes);
    assert_eq!(back.position[3], 5.0); // radius
    assert_eq!(back.color[3], 100.0); // intensity
}

// ═══════════════════════════════════════════════════════════════════════════════
//  RenderError
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn render_error_device_display() {
    let e = RenderError::Device("adapter lost".into());
    let s = format!("{e}");
    assert!(s.contains("GPU device error"), "got: {s}");
    assert!(s.contains("adapter lost"));
}

#[test]
fn render_error_shader_display() {
    let e = RenderError::Shader("naga error".into());
    let s = format!("{e}");
    assert!(s.contains("shader/pipeline error"));
}

#[test]
fn render_error_asset_load_display() {
    let e = RenderError::AssetLoad {
        asset: "mesh".into(),
        detail: "corrupt glTF".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("failed to load mesh"));
    assert!(s.contains("corrupt glTF"));
}

#[test]
fn render_error_material_display() {
    let e = RenderError::Material("missing albedo".into());
    assert!(format!("{e}").contains("material error"));
}

#[test]
fn render_error_shadow_display() {
    let e = RenderError::Shadow("cascade overflow".into());
    assert!(format!("{e}").contains("shadow error"));
}

#[test]
fn render_error_io_from_std() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let e: RenderError = io_err.into();
    let s = format!("{e}");
    assert!(s.contains("I/O error"));
}

#[test]
fn render_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    // RenderError must be thread-safe for async rendering
    assert_send_sync::<RenderError>();
}
