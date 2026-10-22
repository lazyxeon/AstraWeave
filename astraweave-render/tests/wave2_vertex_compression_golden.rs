//! Wave 2 – Golden-value tests for vertex_compression.rs (98 mutants)
//!
//! Targets: OctahedralEncoder (encode, decode, encoding_error),
//!          HalfFloatEncoder (encode, decode, encode_vec2, decode_vec2),
//!          VertexCompressor (compress, decompress, compress_batch, calculate_savings),
//!          CompressedVertex constants.
//!
//! Strategy: Pin EXACT intermediate values (i16, u16, f32) so that any arithmetic
//! mutation (replace *, /, +, -, sign flip, constant change) is immediately caught.

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::vertex_compression::{
    CompressedVertex, HalfFloatEncoder, OctahedralEncoder, VertexCompressor,
};
use glam::{Vec2, Vec3};

// ============================================================================
// OctahedralEncoder::encode — golden i16 outputs
// ============================================================================

// For encode(Vec3::X = (1,0,0)):
//   sum = 1+0+0 = 1, oct = (1, 0), z >= 0 → no wrap
//   x = (1 * 32767).clamp = 32767, y = (0 * 32767) = 0
#[test]
fn oct_encode_pos_x_golden() {
    let enc = OctahedralEncoder::encode(Vec3::X);
    assert_eq!(enc, [32767, 0], "+X axis golden");
}

// For encode(Vec3::NEG_X = (-1,0,0)):
//   sum = 1, oct = (-1, 0), z >= 0
//   x = -32767, y = 0
#[test]
fn oct_encode_neg_x_golden() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_X);
    assert_eq!(enc, [-32767, 0], "-X axis golden");
}

// For encode(Vec3::Y = (0,1,0)):
//   sum = 1, oct = (0, 1), z >= 0
//   x = 0, y = 32767
#[test]
fn oct_encode_pos_y_golden() {
    let enc = OctahedralEncoder::encode(Vec3::Y);
    assert_eq!(enc, [0, 32767], "+Y axis golden");
}

// For encode(Vec3::NEG_Y = (0,-1,0)):
//   sum = 1, oct = (0, -1), z >= 0
//   x = 0, y = -32767
#[test]
fn oct_encode_neg_y_golden() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_Y);
    assert_eq!(enc, [0, -32767], "-Y axis golden");
}

// For encode(Vec3::Z = (0,0,1)):
//   sum = 1, oct = (0, 0), z >= 0
//   x = 0, y = 0
#[test]
fn oct_encode_pos_z_golden() {
    let enc = OctahedralEncoder::encode(Vec3::Z);
    assert_eq!(enc, [0, 0], "+Z axis golden");
}

// For encode(Vec3::NEG_Z = (0,0,-1)):
//   sum = 1, oct = (0, 0), z < 0 → wrap
//   wrapped.x = (1 - |0|) * signum(0) = 1 * 1 = 1
//   wrapped.y = (1 - |0|) * signum(0) = 1 * 1 = 1
//   x = 32767, y = 32767
#[test]
fn oct_encode_neg_z_golden() {
    let enc = OctahedralEncoder::encode(Vec3::NEG_Z);
    assert_eq!(enc, [32767, 32767], "-Z axis golden (wrapping)");
}

// Diagonal: (1/√3, 1/√3, 1/√3)
//   sum = 3/√3 = √3 ≈ 1.7321
//   oct = (1/(√3*√3), 1/(√3*√3)) = (1/3, 1/3)
//   z > 0 → no wrap
//   x = (0.3333 * 32767) ≈ 10922, y = 10922
#[test]
fn oct_encode_diagonal_golden() {
    let n = Vec3::new(1.0, 1.0, 1.0).normalize();
    let enc = OctahedralEncoder::encode(n);
    // 1/√3 / (3/√3) = 1/3 → 0.33333 * 32767 = 10922.33 → 10922 as i16
    assert_eq!(enc[0], enc[1], "Symmetric diagonal should have equal x,y");
    assert!(
        (enc[0] - 10922).abs() <= 1,
        "x should be ~10922, got {}",
        enc[0]
    );
}

// Lower hemisphere diagonal: (1/√3, 1/√3, -1/√3) → z < 0, triggers wrapping
#[test]
fn oct_encode_lower_hemisphere_diagonal_golden() {
    let n = Vec3::new(1.0, 1.0, -1.0).normalize();
    let enc = OctahedralEncoder::encode(n);
    // sum = 3/√3 = √3, oct = (1/3, 1/3), z_component = -1/√3 < 0
    // wrap: x = (1 - |1/3|) * signum(1/3) = (2/3) * 1 = 2/3
    //        y = (1 - |1/3|) * signum(1/3) = 2/3
    // x = (2/3 * 32767) ≈ 21844.67 → 21844 or 21845
    assert!(enc[0] > 0, "Positive x in lower hemisphere");
    assert!(enc[1] > 0, "Positive y in lower hemisphere");
    assert!(
        (enc[0] - 21845).abs() <= 1,
        "x should be ~21845, got {}",
        enc[0]
    );
    assert_eq!(enc[0], enc[1], "Symmetric: x == y");
}

// ============================================================================
// OctahedralEncoder::decode — golden float outputs
// ============================================================================

#[test]
fn oct_decode_pos_x_golden() {
    let dec = OctahedralEncoder::decode([32767, 0]);
    // oct = (1.0, 0.0), z = 1 - 1 - 0 = 0, normalize (1, 0, 0) → (1, 0, 0)
    assert!((dec.x - 1.0).abs() < 0.001);
    assert!(dec.y.abs() < 0.001);
    assert!(dec.z.abs() < 0.001);
}

#[test]
fn oct_decode_pos_y_golden() {
    let dec = OctahedralEncoder::decode([0, 32767]);
    assert!(dec.x.abs() < 0.001);
    assert!((dec.y - 1.0).abs() < 0.001);
    assert!(dec.z.abs() < 0.001);
}

#[test]
fn oct_decode_pos_z_golden() {
    let dec = OctahedralEncoder::decode([0, 0]);
    // oct = (0, 0), z = 1 - 0 - 0 = 1, normalize (0, 0, 1) → (0, 0, 1)
    assert!(dec.x.abs() < 0.001);
    assert!(dec.y.abs() < 0.001);
    assert!((dec.z - 1.0).abs() < 0.001);
}

#[test]
fn oct_decode_neg_z_golden() {
    let dec = OctahedralEncoder::decode([32767, 32767]);
    // oct = (1, 1), z = 1 - 1 - 1 = -1
    // z < 0 → unwrap: x = (1-|1|)*sign(1) = 0, y = 0
    // normalize (0, 0, -1)
    assert!(dec.x.abs() < 0.001);
    assert!(dec.y.abs() < 0.001);
    assert!((dec.z + 1.0).abs() < 0.001, "Should be -1.0, got {}", dec.z);
}

#[test]
fn oct_decode_neg_x_golden() {
    let dec = OctahedralEncoder::decode([-32767, 0]);
    assert!((dec.x + 1.0).abs() < 0.001, "Expected -1.0, got {}", dec.x);
    assert!(dec.y.abs() < 0.001);
    assert!(dec.z.abs() < 0.001);
}

#[test]
fn oct_decode_neg_y_golden() {
    let dec = OctahedralEncoder::decode([0, -32767]);
    assert!(dec.x.abs() < 0.001);
    assert!((dec.y + 1.0).abs() < 0.001, "Expected -1.0, got {}", dec.y);
    assert!(dec.z.abs() < 0.001);
}

// ============================================================================
// OctahedralEncoder::encoding_error — golden error values
// ============================================================================

#[test]
fn oct_encoding_error_axis_is_near_zero() {
    // Axis-aligned normals should have ~zero error after roundtrip
    for axis in [Vec3::X, Vec3::Y, Vec3::NEG_X, Vec3::NEG_Y] {
        let err = OctahedralEncoder::encoding_error(axis);
        assert!(err < 0.0005, "Axis {:?} error {} should be ~0", axis, err);
    }
}

#[test]
fn oct_encoding_error_is_nonnegative() {
    // acos returns [0, π], so error is always >= 0
    let normals = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::NEG_X,
        Vec3::NEG_Y,
        Vec3::NEG_Z,
        Vec3::new(0.3, 0.5, 0.8).normalize(),
        Vec3::new(-0.7, 0.2, -0.3).normalize(),
    ];
    for n in normals {
        let err = OctahedralEncoder::encoding_error(n);
        assert!(
            err >= 0.0,
            "Error should be non-negative for {:?}, got {}",
            n,
            err
        );
    }
}

#[test]
fn oct_encoding_error_off_axis_is_small_but_nonzero() {
    let n = Vec3::new(0.3, 0.5, 0.8).normalize();
    let err = OctahedralEncoder::encoding_error(n);
    assert!(err > 0.0, "Off-axis should have nonzero quantization error");
    assert!(
        err < 0.02,
        "Error should be small (< 0.02 rad), got {}",
        err
    );
}

#[test]
fn oct_encoding_error_below_one_degree_for_all_octants() {
    // Test one normal per octant (all 8 sign combinations)
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                let n = Vec3::new(sx * 0.5, sy * 0.3, sz * 0.8).normalize();
                let err = OctahedralEncoder::encoding_error(n);
                // 1 degree = 0.01745 rad
                assert!(
                    err < 0.02,
                    "Error {} > 1 deg for ({}, {}, {})",
                    err,
                    sx,
                    sy,
                    sz
                );
            }
        }
    }
}

// ============================================================================
// OctahedralEncoder roundtrip — exact intermediate check
// ============================================================================

#[test]
fn oct_roundtrip_all_axes_exact() {
    let cases: &[(Vec3, [i16; 2])] = &[
        (Vec3::X, [32767, 0]),
        (Vec3::Y, [0, 32767]),
        (Vec3::Z, [0, 0]),
        (Vec3::NEG_X, [-32767, 0]),
        (Vec3::NEG_Y, [0, -32767]),
        (Vec3::NEG_Z, [32767, 32767]),
    ];
    for &(normal, expected_enc) in cases {
        let enc = OctahedralEncoder::encode(normal);
        assert_eq!(enc, expected_enc, "Encode {:?}", normal);
        let dec = OctahedralEncoder::decode(enc);
        assert!(
            (dec - normal).length() < 0.01,
            "Decode {:?} → {:?}, expected {:?}",
            enc,
            dec,
            normal
        );
    }
}

// ============================================================================
// HalfFloatEncoder::encode — golden u16 outputs (IEEE 754 half-precision)
// ============================================================================

#[test]
fn half_encode_zero_golden() {
    assert_eq!(HalfFloatEncoder::encode(0.0), 0x0000);
}

#[test]
fn half_encode_one_golden() {
    // 1.0 = 2^0 → exponent = 15, mantissa = 0 → 0_01111_0000000000 = 0x3C00
    assert_eq!(HalfFloatEncoder::encode(1.0), 0x3C00);
}

#[test]
fn half_encode_half_golden() {
    // 0.5 = 2^(-1) → exponent = 14 → 0_01110_0000000000 = 0x3800
    assert_eq!(HalfFloatEncoder::encode(0.5), 0x3800);
}

#[test]
fn half_encode_quarter_golden() {
    // 0.25 = 2^(-2) → exponent = 13 → 0_01101_0000000000 = 0x3400
    assert_eq!(HalfFloatEncoder::encode(0.25), 0x3400);
}

#[test]
fn half_encode_three_quarters_golden() {
    // 0.75 = 1.5 * 2^(-1) → exp=14, mantissa=1000000000 → 0_01110_1000000000 = 0x3A00
    assert_eq!(HalfFloatEncoder::encode(0.75), 0x3A00);
}

#[test]
fn half_encode_neg_one_golden() {
    // -1.0 → sign=1, exp=15, mantissa=0 → 1_01111_0000000000 = 0xBC00
    assert_eq!(HalfFloatEncoder::encode(-1.0), 0xBC00);
}

// ============================================================================
// HalfFloatEncoder::decode — golden float outputs
// ============================================================================

#[test]
fn half_decode_zero_golden() {
    assert_eq!(HalfFloatEncoder::decode(0x0000), 0.0);
}

#[test]
fn half_decode_one_golden() {
    assert_eq!(HalfFloatEncoder::decode(0x3C00), 1.0);
}

#[test]
fn half_decode_half_golden() {
    assert_eq!(HalfFloatEncoder::decode(0x3800), 0.5);
}

#[test]
fn half_decode_neg_one_golden() {
    assert_eq!(HalfFloatEncoder::decode(0xBC00), -1.0);
}

// ============================================================================
// HalfFloatEncoder::encode_vec2 / decode_vec2
// ============================================================================

#[test]
fn half_encode_vec2_golden() {
    let uv = Vec2::new(0.5, 1.0);
    let enc = HalfFloatEncoder::encode_vec2(uv);
    assert_eq!(enc[0], 0x3800, "u = 0.5");
    assert_eq!(enc[1], 0x3C00, "v = 1.0");
}

#[test]
fn half_decode_vec2_golden() {
    let dec = HalfFloatEncoder::decode_vec2([0x3800, 0x3C00]);
    assert_eq!(dec.x, 0.5);
    assert_eq!(dec.y, 1.0);
}

#[test]
fn half_vec2_roundtrip_preserves_order() {
    // Ensure x→[0] and y→[1], not swapped
    let uv = Vec2::new(0.25, 0.75);
    let enc = HalfFloatEncoder::encode_vec2(uv);
    assert_eq!(enc[0], 0x3400, "x=0.25 must be in [0]");
    assert_eq!(enc[1], 0x3A00, "y=0.75 must be in [1]");
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert_eq!(dec.x, 0.25);
    assert_eq!(dec.y, 0.75);
}

// ============================================================================
// VertexCompressor::compress / decompress — field layout verification
// ============================================================================

#[test]
fn compress_position_is_exact() {
    // Position is stored as f32 without compression
    let pos = Vec3::new(1.5, -2.7, 300.0);
    let cv = VertexCompressor::compress(pos, Vec3::Y, Vec2::ZERO);
    let (dec_pos, _, _) = VertexCompressor::decompress(&cv);
    assert_eq!(dec_pos, pos, "Position should be bit-exact");
}

#[test]
fn compress_normal_uses_octahedral() {
    let normal = Vec3::X;
    let cv = VertexCompressor::compress(Vec3::ZERO, normal, Vec2::ZERO);
    assert_eq!(
        cv.normal_oct,
        [32767, 0],
        "Normal should use octahedral encoding"
    );
}

#[test]
fn compress_uv_uses_half_float() {
    let uv = Vec2::new(0.5, 1.0);
    let cv = VertexCompressor::compress(Vec3::ZERO, Vec3::Y, uv);
    assert_eq!(
        cv.uv_half,
        [0x3800, 0x3C00],
        "UV should use half-float encoding"
    );
}

#[test]
fn decompress_recovers_all_fields() {
    let pos = Vec3::new(10.0, 20.0, 30.0);
    let normal = Vec3::Y;
    let uv = Vec2::new(0.25, 0.75);
    let cv = VertexCompressor::compress(pos, normal, uv);
    let (d_pos, d_norm, d_uv) = VertexCompressor::decompress(&cv);

    assert_eq!(d_pos, pos);
    assert!((d_norm - normal).length() < 0.01);
    assert!((d_uv.x - 0.25).abs() < 0.001);
    assert!((d_uv.y - 0.75).abs() < 0.001);
}

// ============================================================================
// VertexCompressor::compress_batch
// ============================================================================

#[test]
fn compress_batch_correct_count() {
    let n = 5;
    let positions: Vec<Vec3> = (0..n).map(|i| Vec3::new(i as f32, 0.0, 0.0)).collect();
    let normals: Vec<Vec3> = vec![Vec3::Y; n];
    let uvs: Vec<Vec2> = vec![Vec2::ZERO; n];
    let result = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    assert_eq!(result.len(), n, "Batch output count must match input");
}

#[test]
fn compress_batch_empty() {
    let result = VertexCompressor::compress_batch(&[], &[], &[]);
    assert!(result.is_empty());
}

#[test]
fn compress_batch_preserves_order() {
    let positions = vec![
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
        Vec3::new(3.0, 0.0, 0.0),
    ];
    let normals = vec![Vec3::Y; 3];
    let uvs = vec![Vec2::ZERO; 3];
    let result = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    for (i, cv) in result.iter().enumerate() {
        assert_eq!(
            cv.position[0],
            (i + 1) as f32,
            "Position order must be preserved"
        );
    }
}

#[test]
#[should_panic(expected = "Position and normal counts must match")]
fn compress_batch_panics_on_normal_length_mismatch() {
    let positions = vec![Vec3::ZERO; 3];
    let normals = vec![Vec3::Y; 2]; // WRONG length
    let uvs = vec![Vec2::ZERO; 3];
    let _ = VertexCompressor::compress_batch(&positions, &normals, &uvs);
}

#[test]
#[should_panic(expected = "Position and UV counts must match")]
fn compress_batch_panics_on_uv_length_mismatch() {
    let positions = vec![Vec3::ZERO; 3];
    let normals = vec![Vec3::Y; 3];
    let uvs = vec![Vec2::ZERO; 2]; // WRONG length
    let _ = VertexCompressor::compress_batch(&positions, &normals, &uvs);
}

// ============================================================================
// VertexCompressor::calculate_savings — golden arithmetic
// ============================================================================

#[test]
fn calculate_savings_1_vertex_golden() {
    let (std, comp, save, pct) = VertexCompressor::calculate_savings(1);
    assert_eq!(std, 32);
    assert_eq!(comp, 20);
    assert_eq!(save, 12);
    assert!((pct - 37.5).abs() < 0.01, "Expected 37.5%, got {}", pct);
}

#[test]
fn calculate_savings_1000_vertices_golden() {
    let (std, comp, save, pct) = VertexCompressor::calculate_savings(1000);
    assert_eq!(std, 32_000);
    assert_eq!(comp, 20_000);
    assert_eq!(save, 12_000);
    assert!((pct - 37.5).abs() < 0.01);
}

#[test]
fn calculate_savings_standard_uses_32_bytes() {
    let (std, _, _, _) = VertexCompressor::calculate_savings(100);
    assert_eq!(std, 100 * 32, "Standard size = vertex_count * 32");
}

#[test]
fn calculate_savings_compressed_uses_20_bytes() {
    let (_, comp, _, _) = VertexCompressor::calculate_savings(100);
    assert_eq!(comp, 100 * 20, "Compressed size = vertex_count * 20");
}

#[test]
fn calculate_savings_is_difference() {
    let (std, comp, save, _) = VertexCompressor::calculate_savings(42);
    assert_eq!(save, std - comp, "Savings = standard - compressed");
}

#[test]
fn calculate_savings_percent_formula() {
    let (std, _, save, pct) = VertexCompressor::calculate_savings(500);
    let expected_pct = (save as f32 / std as f32) * 100.0;
    assert!((pct - expected_pct).abs() < 0.001);
}

// ============================================================================
// CompressedVertex constants
// ============================================================================

#[test]
fn compressed_vertex_standard_size_is_32() {
    assert_eq!(CompressedVertex::STANDARD_SIZE, 32);
}

#[test]
fn compressed_vertex_compressed_size_is_20() {
    assert_eq!(CompressedVertex::COMPRESSED_SIZE, 20);
}

#[test]
fn compressed_vertex_memory_reduction_golden() {
    // 1 - 20/32 = 0.375
    assert!((CompressedVertex::MEMORY_REDUCTION - 0.375).abs() < 0.001);
}

#[test]
fn compressed_vertex_struct_size_matches_constant() {
    assert_eq!(
        std::mem::size_of::<CompressedVertex>(),
        CompressedVertex::COMPRESSED_SIZE,
    );
}

// ============================================================================
// Edge cases — quantization boundaries
// ============================================================================

#[test]
fn oct_encode_near_zero_normal_z_axis() {
    // Normal with very small x, y components — should encode to near [0, 0]
    let n = Vec3::new(0.001, 0.001, 1.0).normalize();
    let enc = OctahedralEncoder::encode(n);
    assert!(
        enc[0].abs() < 100,
        "Near +Z should have small x, got {}",
        enc[0]
    );
    assert!(
        enc[1].abs() < 100,
        "Near +Z should have small y, got {}",
        enc[1]
    );
}

#[test]
fn oct_encode_exactly_on_seam() {
    // Normal on the xz plane: (1, 0, 0) → [32767, 0], z=0 is NOT < 0
    let n = Vec3::new(1.0, 0.0, 0.0);
    let enc = OctahedralEncoder::encode(n);
    let dec = OctahedralEncoder::decode(enc);
    assert!(
        (dec - n).length() < 0.01,
        "Seam normal should round-trip cleanly"
    );
}

#[test]
fn half_float_large_uv_roundtrip() {
    // UV can exceed [0,1] (e.g., tiling)
    let uv = Vec2::new(4.0, 8.0);
    let enc = HalfFloatEncoder::encode_vec2(uv);
    let dec = HalfFloatEncoder::decode_vec2(enc);
    assert!((dec.x - 4.0).abs() < 0.01, "Large UV roundtrip x");
    assert!((dec.y - 8.0).abs() < 0.01, "Large UV roundtrip y");
}

#[test]
fn half_float_subnormal() {
    // Very small value — half-float subnormal territory
    let val = 0.00003;
    let enc = HalfFloatEncoder::encode(val);
    let dec = HalfFloatEncoder::decode(enc);
    // Subnormal precision is limited, but value should be close-ish
    assert!(
        (dec - val).abs() < 0.0001,
        "Subnormal roundtrip: {} vs {}",
        dec,
        val
    );
}

#[test]
fn half_float_negative_uv_golden() {
    let val = -0.5;
    let enc = HalfFloatEncoder::encode(val);
    // -0.5 = -(2^(-1)) → sign=1, exp=14, mantissa=0 → 0xB800
    assert_eq!(enc, 0xB800, "Negative half golden");
    assert_eq!(HalfFloatEncoder::decode(enc), -0.5);
}

// ============================================================================
// Composition: compress then decompress batch
// ============================================================================

#[test]
fn batch_roundtrip_positions_exact() {
    let positions: Vec<Vec3> = (0..10)
        .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * -0.2, i as f32 * 0.5))
        .collect();
    let normals = vec![Vec3::Y; 10];
    let uvs = vec![Vec2::new(0.5, 0.5); 10];
    let compressed = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    for (i, cv) in compressed.iter().enumerate() {
        let (pos, _, _) = VertexCompressor::decompress(cv);
        assert_eq!(pos, positions[i], "Batch position {} must be exact", i);
    }
}

#[test]
fn batch_roundtrip_normals_close() {
    let normals: Vec<Vec3> = [
        Vec3::X,
        Vec3::Y,
        Vec3::Z,
        Vec3::NEG_X,
        Vec3::NEG_Y,
        Vec3::NEG_Z,
    ]
    .to_vec();
    let n = normals.len();
    let positions = vec![Vec3::ZERO; n];
    let uvs = vec![Vec2::ZERO; n];
    let compressed = VertexCompressor::compress_batch(&positions, &normals, &uvs);
    for (i, cv) in compressed.iter().enumerate() {
        let (_, dec_n, _) = VertexCompressor::decompress(cv);
        assert!(
            (dec_n - normals[i]).length() < 0.02,
            "Normal {} roundtrip error too high: {:?} vs {:?}",
            i,
            dec_n,
            normals[i]
        );
    }
}
