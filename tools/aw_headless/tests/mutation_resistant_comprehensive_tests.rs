//! Mutation-resistant comprehensive tests for aw_headless.
//!
//! Tests pure utility functions: wrap_fs_into_fullscreen, srgb_encode_u8, image_delta.
//! (render_wgsl_to_image requires GPU — skipped in headless CI.)

use aw_headless::*;

// ═══════════════════════════════════════════════════════════════════════════
// wrap_fs_into_fullscreen
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wrap_fs_contains_module_body() {
    let body = "@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }";
    let result = wrap_fs_into_fullscreen(body);
    assert!(
        result.contains(body),
        "should contain the module body verbatim"
    );
}

#[test]
fn wrap_fs_contains_vertex_shader() {
    let result = wrap_fs_into_fullscreen("");
    assert!(result.contains("@vertex"), "should contain a vertex shader");
    assert!(result.contains("vs_main"), "should contain vs_main");
}

#[test]
fn wrap_fs_contains_fullscreen_triangle() {
    let result = wrap_fs_into_fullscreen("");
    assert!(
        result.contains("vertex_index"),
        "should use vertex_index builtin"
    );
}

#[test]
fn wrap_fs_contains_uv_output() {
    let result = wrap_fs_into_fullscreen("");
    assert!(result.contains("uv"), "should output UV coordinates");
}

#[test]
fn wrap_fs_empty_body() {
    let result = wrap_fs_into_fullscreen("");
    assert!(!result.is_empty());
}

#[test]
fn wrap_fs_with_whitespace() {
    let result = wrap_fs_into_fullscreen("  \n  ");
    assert!(result.contains("  \n  "));
}

// ═══════════════════════════════════════════════════════════════════════════
// srgb_encode_u8
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn srgb_encode_zero_is_zero() {
    assert_eq!(srgb_encode_u8(0.0), 0);
}

#[test]
fn srgb_encode_one_is_255() {
    assert_eq!(srgb_encode_u8(1.0), 255);
}

#[test]
fn srgb_encode_clamps_negative() {
    assert_eq!(srgb_encode_u8(-1.0), 0);
}

#[test]
fn srgb_encode_clamps_above_one() {
    assert_eq!(srgb_encode_u8(2.0), 255);
}

#[test]
fn srgb_encode_linear_threshold() {
    // Below the linear threshold (0.0031308), sRGB = 12.92 * linear
    let linear: f32 = 0.001;
    let expected_srgb = (12.92_f32 * linear * 255.0 + 0.5).floor() as u8;
    assert_eq!(srgb_encode_u8(linear), expected_srgb);
}

#[test]
fn srgb_encode_midpoint_reasonable() {
    // Linear 0.5 → sRGB should be ~188 (sqrt-ish curve)
    let v = srgb_encode_u8(0.5);
    assert!((180..=200).contains(&v), "srgb(0.5) = {v}, expected ~188");
}

#[test]
fn srgb_encode_monotonic() {
    let mut prev = 0u8;
    for i in 0..=100 {
        let linear = i as f32 / 100.0;
        let srgb = srgb_encode_u8(linear);
        assert!(
            srgb >= prev,
            "srgb should be monotonically increasing: {srgb} < {prev} at {linear}"
        );
        prev = srgb;
    }
}

#[test]
fn srgb_encode_quarter() {
    // Linear 0.25 → should be somewhere around 137 (sRGB is not linear)
    let v = srgb_encode_u8(0.25);
    assert!((120..=150).contains(&v), "srgb(0.25) = {v}");
}

// ═══════════════════════════════════════════════════════════════════════════
// image_delta
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn image_delta_identical_is_zero() {
    let a = vec![10, 20, 30, 255];
    let (max_d, avg_d) = image_delta(&a, &a);
    assert_eq!(max_d, 0);
    assert!((avg_d - 0.0).abs() < f32::EPSILON);
}

#[test]
fn image_delta_max_difference() {
    let a = vec![0, 0, 0, 0];
    let b = vec![255, 255, 255, 255];
    let (max_d, avg_d) = image_delta(&a, &b);
    assert_eq!(max_d, 255);
    assert!((avg_d - 255.0).abs() < f32::EPSILON);
}

#[test]
fn image_delta_single_channel_diff() {
    let a = vec![100, 100, 100, 100];
    let b = vec![150, 100, 100, 100];
    let (max_d, avg_d) = image_delta(&a, &b);
    assert_eq!(max_d, 50);
    // avg = 50/4 = 12.5
    assert!((avg_d - 12.5).abs() < 0.01, "avg_d = {avg_d}");
}

#[test]
fn image_delta_symmetry() {
    let a = vec![10, 20, 30, 40];
    let b = vec![50, 60, 70, 80];
    let (max1, avg1) = image_delta(&a, &b);
    let (max2, avg2) = image_delta(&b, &a);
    assert_eq!(max1, max2);
    assert!((avg1 - avg2).abs() < f32::EPSILON);
}

#[test]
fn image_delta_empty_buffers() {
    let a: Vec<u8> = vec![];
    let (max_d, _avg_d) = image_delta(&a, &a);
    assert_eq!(max_d, 0);
}

#[test]
fn image_delta_large_buffer() {
    let n = 4096;
    let a = vec![128u8; n];
    let b = vec![130u8; n];
    let (max_d, avg_d) = image_delta(&a, &b);
    assert_eq!(max_d, 2);
    assert!((avg_d - 2.0).abs() < 0.01);
}

#[test]
fn image_delta_mixed() {
    let a = vec![0, 100, 200, 50];
    let b = vec![10, 90, 210, 50];
    let (max_d, avg_d) = image_delta(&a, &b);
    assert_eq!(max_d, 10); // max(10,10,10,0) = 10
                           // avg = (10+10+10+0)/4 = 7.5
    assert!((avg_d - 7.5).abs() < 0.01, "avg_d = {avg_d}");
}
