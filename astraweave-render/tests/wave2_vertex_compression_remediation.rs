//! Wave 2 proactive remediation tests for vertex_compression.rs (98 mutants, 0 external tests).
//!
//! Targets:
//!   - OctahedralEncoder encode/decode roundtripfor many normals
//!   - OctahedralEncoder edge cases: near-zero components, lower hemisphere wrapping
//!   - HalfFloatEncoder: precise values, large values, near-zero, negative
//!   - VertexCompressor: compress/decompress golden values
//!   - VertexCompressor::compress_batch with empty arrays
//!   - calculate_savings zero/one/large vertex counts
//!   - CompressedVertex constants

use astraweave_render::vertex_compression::{
    CompressedVertex, HalfFloatEncoder, OctahedralEncoder, VertexCompressor,
};
use glam::{Vec2, Vec3};

const MAX_ANGULAR_ERROR: f32 = 0.02; // ~1.1 degrees

// ══════════════════════════════════════════════════════════════════════════════
// Octahedral encoding — systematic sweep
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn oct_roundtrip_positive_x() {
    let enc = OctahedralEncoder::encode(Vec3::X);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::X).length() < 0.01);
}

#[test]
fn oct_roundtrip_positive_y() {
    let enc = OctahedralEncoder::encode(Vec3::Y);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::Y).length() < 0.01);
}

#[test]
fn oct_roundtrip_positive_z() {
    let enc = OctahedralEncoder::encode(Vec3::Z);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::Z).length() < 0.01);
}

#[test]
fn oct_roundtrip_neg_x() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_X);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::NEG_X).length() < 0.02);
}

#[test]
fn oct_roundtrip_neg_y() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_Y);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::NEG_Y).length() < 0.02);
}

#[test]
fn oct_roundtrip_neg_z() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_Z);
    let dec = OctahedralEncoder::decode(enc);
    assert!((dec - Vec3::NEG_Z).length() < 0.02);
}

#[test]
fn oct_upper_hemisphere_sweep() {
    for i in 0..16 {
        let phi = i as f32 * std::f32::consts::TAU / 16.0;
        for j in 1..5 {
            let theta = j as f32 * std::f32::consts::FRAC_PI_2 / 5.0;
            let n = Vec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            )
            .normalize();
            let error = OctahedralEncoder::encoding_error(n);
            assert!(
                error < MAX_ANGULAR_ERROR,
                "upper sweep ({i},{j}) error={error}"
            );
        }
    }
}

#[test]
fn oct_lower_hemisphere_sweep() {
    for i in 0..16 {
        let phi = i as f32 * std::f32::consts::TAU / 16.0;
        for j in 1..5 {
            let theta = std::f32::consts::FRAC_PI_2 + j as f32 * std::f32::consts::FRAC_PI_2 / 5.0;
            let n = Vec3::new(
                theta.sin() * phi.cos(),
                theta.sin() * phi.sin(),
                theta.cos(),
            )
            .normalize();
            let error = OctahedralEncoder::encoding_error(n);
            assert!(error < 0.03, "lower sweep ({i},{j}) error={error}, n={n:?}");
        }
    }
}

#[test]
fn oct_equator_ring() {
    // z ≈ 0 is the boundary between upper and lower hemisphere wrap
    for i in 0..32 {
        let phi = i as f32 * std::f32::consts::TAU / 32.0;
        let n = Vec3::new(phi.cos(), phi.sin(), 0.0).normalize();
        let error = OctahedralEncoder::encoding_error(n);
        assert!(error < 0.03, "equator i={i} error={error}");
    }
}

#[test]
fn oct_diagonal_normals() {
    let diags = [
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(-1.0, -1.0, -1.0),
    ];
    for d in &diags {
        let n = d.normalize();
        let error = OctahedralEncoder::encoding_error(n);
        assert!(error < 0.02, "diagonal {n:?} error={error}");
    }
}

#[test]
fn oct_encoded_values_range() {
    // Encoded values should be in [-32767, 32767]
    for _ in 0..100 {
        let n = Vec3::new(
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
            rand::random::<f32>() * 2.0 - 1.0,
        )
        .normalize();
        if n.is_nan() {
            continue;
        }
        let [x, y] = OctahedralEncoder::encode(n);
        assert!(x >= -32767, "x={x} out of range");
        assert!(y >= -32767, "y={y} out of range");
    }
}

#[test]
fn oct_decoded_is_normalized() {
    let test_encoded = [
        [0i16, 32767],
        [-32767, 0],
        [16000, 16000],
        [0, 0],
        [-10000, 20000],
    ];
    for enc in &test_encoded {
        let dec = OctahedralEncoder::decode(*enc);
        assert!(
            (dec.length() - 1.0).abs() < 0.01,
            "decoded {:?} length={} should be ~1",
            enc,
            dec.length()
        );
    }
}

#[test]
fn oct_encoding_error_zero_for_perfect_axis() {
    // Principal axes can be encoded exactly (or near-exactly)
    for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
        let error = OctahedralEncoder::encoding_error(axis);
        assert!(error < 0.001, "axis {axis:?} error={error} should be tiny");
    }
}

#[test]
fn oct_sign_preservation() {
    // Encoding should preserve the sign relationship
    let test_normals = [
        Vec3::new(0.5, 0.5, 0.5).normalize(),
        Vec3::new(-0.5, 0.5, 0.5).normalize(),
        Vec3::new(0.5, -0.5, 0.5).normalize(),
        Vec3::new(0.5, 0.5, -0.5).normalize(),
    ];
    for n in &test_normals {
        let dec = OctahedralEncoder::decode(OctahedralEncoder::encode(*n));
        assert_eq!(
            n.x.signum() as i32,
            dec.x.signum() as i32,
            "x sign mismatch for {n:?}"
        );
        assert_eq!(
            n.y.signum() as i32,
            dec.y.signum() as i32,
            "y sign mismatch for {n:?}"
        );
        assert_eq!(
            n.z.signum() as i32,
            dec.z.signum() as i32,
            "z sign mismatch for {n:?}"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// HalfFloatEncoder
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn half_encode_zero() {
    let enc = HalfFloatEncoder::encode(0.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert_eq!(dec, 0.0);
}

#[test]
fn half_encode_one() {
    let enc = HalfFloatEncoder::encode(1.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 1.0).abs() < 0.001);
}

#[test]
fn half_encode_negative() {
    let enc = HalfFloatEncoder::encode(-2.5);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec + 2.5).abs() < 0.01);
}

#[test]
fn half_encode_small_positive() {
    let enc = HalfFloatEncoder::encode(0.001);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 0.001).abs() < 0.001);
}

#[test]
fn half_encode_large_value() {
    let enc = HalfFloatEncoder::encode(60000.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert!((dec - 60000.0).abs() < 100.0, "large value: {dec}");
}

#[test]
fn half_encode_uv_typical_range() {
    for i in 0..=10 {
        let v = i as f32 * 0.1;
        let enc = HalfFloatEncoder::encode(v);
        let dec = HalfFloatEncoder::decode(enc);
        assert!(
            (dec - v).abs() < 0.001,
            "UV value {v} roundtripped to {dec}"
        );
    }
}

#[test]
fn half_vec2_roundtrip() {
    let uv = Vec2::new(0.25, 0.75);
    let enc = HalfFloatEncoder::encode_vec2(uv);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert!((dec.x - 0.25).abs() < 0.001);
    assert!((dec.y - 0.75).abs() < 0.001);
}

#[test]
fn half_vec2_zero() {
    let uv = Vec2::ZERO;
    let enc = HalfFloatEncoder::encode_vec2(uv);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert_eq!(dec, Vec2::ZERO);
}

#[test]
fn half_vec2_one_one() {
    let uv = Vec2::ONE;
    let enc = HalfFloatEncoder::encode_vec2(uv);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert!((dec.x - 1.0).abs() < 0.001);
    assert!((dec.y - 1.0).abs() < 0.001);
}

#[test]
fn half_negative_zero() {
    let enc = HalfFloatEncoder::encode(-0.0);
    let dec = HalfFloatEncoder::decode(enc);
    assert_eq!(dec, 0.0); // -0.0 == 0.0 in float comparison
}

// ══════════════════════════════════════════════════════════════════════════════
// VertexCompressor
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn compress_position_exact() {
    let pos = Vec3::new(1.23, 4.56, 7.89);
    let cv = VertexCompressor::compress(pos, Vec3::Y, Vec2::ZERO);
    assert_eq!(cv.position, pos.to_array());
}

#[test]
fn compress_normal_close() {
    let norm = Vec3::new(0.0, 0.0, 1.0);
    let cv = VertexCompressor::compress(Vec3::ZERO, norm, Vec2::ZERO);
    let (_, dec_n, _) = VertexCompressor::decompress(&cv);
    assert!((dec_n - norm).length() < 0.02);
}

#[test]
fn compress_uv_close() {
    let uv = Vec2::new(0.3, 0.7);
    let cv = VertexCompressor::compress(Vec3::ZERO, Vec3::Y, uv);
    let (_, _, dec_uv) = VertexCompressor::decompress(&cv);
    assert!((dec_uv.x - 0.3).abs() < 0.001);
    assert!((dec_uv.y - 0.7).abs() < 0.001);
}

#[test]
fn decompress_position_unchanged() {
    let pos = Vec3::new(-10.0, 200.0, 0.001);
    let cv = VertexCompressor::compress(pos, Vec3::Y, Vec2::ZERO);
    let (dec_pos, _, _) = VertexCompressor::decompress(&cv);
    assert_eq!(dec_pos, pos, "position should be exact (no compression)");
}

#[test]
fn compress_batch_empty() {
    let result = VertexCompressor::compress_batch(&[], &[], &[]);
    assert!(result.is_empty());
}

#[test]
fn compress_batch_single() {
    let result = VertexCompressor::compress_batch(
        &[Vec3::new(1.0, 2.0, 3.0)],
        &[Vec3::Y],
        &[Vec2::new(0.5, 0.5)],
    );
    assert_eq!(result.len(), 1);
    let (pos, _, _) = VertexCompressor::decompress(&result[0]);
    assert_eq!(pos, Vec3::new(1.0, 2.0, 3.0));
}

#[test]
fn compress_batch_preserves_ordering() {
    let positions = vec![Vec3::X, Vec3::Y, Vec3::Z];
    let normals = vec![Vec3::Z, Vec3::Z, Vec3::Z];
    let uvs = vec![
        Vec2::new(0.1, 0.0),
        Vec2::new(0.2, 0.0),
        Vec2::new(0.3, 0.0),
    ];
    let batch = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    assert_eq!(batch.len(), 3);
    for (i, cv) in batch.iter().enumerate() {
        let (pos, _, _) = VertexCompressor::decompress(cv);
        assert_eq!(pos, positions[i], "vertex {i} position mismatch");
    }
}

#[test]
#[should_panic(expected = "Position and normal counts must match")]
fn compress_batch_mismatched_normals_panics() {
    VertexCompressor::compress_batch(
        &[Vec3::ZERO, Vec3::ONE],
        &[Vec3::Y], // only 1 normal for 2 positions
        &[Vec2::ZERO, Vec2::ONE],
    );
}

#[test]
#[should_panic(expected = "Position and UV counts must match")]
fn compress_batch_mismatched_uvs_panics() {
    VertexCompressor::compress_batch(
        &[Vec3::ZERO, Vec3::ONE],
        &[Vec3::Y, Vec3::Y],
        &[Vec2::ZERO], // only 1 UV for 2 positions
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// calculate_savings
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn savings_zero_vertices() {
    let (std, comp, save, _pct) = VertexCompressor::calculate_savings(0);
    assert_eq!(std, 0);
    assert_eq!(comp, 0);
    assert_eq!(save, 0);
}

#[test]
fn savings_one_vertex() {
    let (std, comp, save, pct) = VertexCompressor::calculate_savings(1);
    assert_eq!(std, 32);
    assert_eq!(comp, 20);
    assert_eq!(save, 12);
    assert!((pct - 37.5).abs() < 0.1);
}

#[test]
fn savings_1000_vertices() {
    let (std, comp, save, pct) = VertexCompressor::calculate_savings(1000);
    assert_eq!(std, 32000);
    assert_eq!(comp, 20000);
    assert_eq!(save, 12000);
    assert!((pct - 37.5).abs() < 0.1);
}

// ══════════════════════════════════════════════════════════════════════════════
// CompressedVertex constants
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn compressed_vertex_standard_size() {
    assert_eq!(CompressedVertex::STANDARD_SIZE, 32);
}

#[test]
fn compressed_vertex_compressed_size() {
    assert_eq!(CompressedVertex::COMPRESSED_SIZE, 20);
}

#[test]
fn compressed_vertex_memory_reduction() {
    assert!((CompressedVertex::MEMORY_REDUCTION - 0.375).abs() < 0.001);
}

#[test]
fn compressed_vertex_struct_size() {
    assert_eq!(std::mem::size_of::<CompressedVertex>(), 20);
}

#[test]
fn compressed_vertex_reduction_matches_sizes() {
    let expected =
        1.0 - (CompressedVertex::COMPRESSED_SIZE as f32 / CompressedVertex::STANDARD_SIZE as f32);
    assert!((expected - CompressedVertex::MEMORY_REDUCTION).abs() < 0.001);
}
