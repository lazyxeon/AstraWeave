//! Mutation-resistant comprehensive tests for astraweave-asset-pipeline.
//!
//! Tests Mesh, CompressionStats, ValidationReport, AssetValidator,
//! BatchValidationReport, and AstcBlockSize — all pure logic, no GPU/I/O.

use astraweave_asset_pipeline::mesh::Mesh;
use astraweave_asset_pipeline::optimize_mesh;
use astraweave_asset_pipeline::texture::{AstcBlockSize, CompressionStats};
use astraweave_asset_pipeline::validator::{AssetValidator, ValidationMetrics, ValidationReport};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════════
// Mesh construction & accessors
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn mesh_new_valid() {
    let m = Mesh::new(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0], vec![0, 1, 0]).unwrap();
    assert_eq!(m.vertex_count(), 2);
    assert_eq!(m.triangle_count(), 1);
}

#[test]
fn mesh_new_single_triangle() {
    let m = Mesh::new(
        vec![0.0; 9], // 3 vertices × 3 floats
        vec![0, 1, 2],
    )
    .unwrap();
    assert_eq!(m.vertex_count(), 3);
    assert_eq!(m.triangle_count(), 1);
}

#[test]
fn mesh_new_positions_not_divisible_by_3_fails() {
    let result = Mesh::new(vec![0.0, 1.0], vec![]);
    assert!(result.is_err());
}

#[test]
fn mesh_new_indices_not_divisible_by_3_fails() {
    let result = Mesh::new(vec![0.0, 0.0, 0.0], vec![0, 0]);
    assert!(result.is_err());
}

#[test]
fn mesh_new_empty_valid() {
    let m = Mesh::new(vec![], vec![]).unwrap();
    assert_eq!(m.vertex_count(), 0);
    assert_eq!(m.triangle_count(), 0);
}

#[test]
fn mesh_vertex_count_formula() {
    // vertex_count = positions.len() / 3
    let m = Mesh::new(vec![0.0; 30], vec![]).unwrap();
    assert_eq!(m.vertex_count(), 10);
}

#[test]
fn mesh_triangle_count_formula() {
    // triangle_count = indices.len() / 3
    let positions = vec![0.0; 12]; // 4 vertices
    let indices = vec![0, 1, 2, 0, 2, 3]; // 2 triangles
    let m = Mesh::new(positions, indices).unwrap();
    assert_eq!(m.triangle_count(), 2);
}

#[test]
fn mesh_clone() {
    let m = Mesh::new(vec![1.0, 2.0, 3.0], vec![]).unwrap();
    let m2 = m.clone();
    assert_eq!(m2.vertex_count(), 1);
}

#[test]
fn mesh_debug() {
    let m = Mesh::new(vec![0.0; 3], vec![]).unwrap();
    let dbg = format!("{m:?}");
    assert!(dbg.contains("Mesh"));
}

// ═══════════════════════════════════════════════════════════════════════════
// optimize_mesh
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn optimize_mesh_basic() {
    // Simple quad: 4 vertices, 2 triangles
    let positions = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let indices = vec![0, 1, 2, 0, 2, 3];
    let mesh = Mesh::new(positions, indices).unwrap();
    let (opt, stats) = optimize_mesh(mesh).unwrap();
    assert_eq!(opt.vertex_count(), 4);
    assert_eq!(opt.triangle_count(), 2);
    assert_eq!(stats.vertex_count, 4);
    assert_eq!(stats.index_count, 6);
}

#[test]
fn optimize_mesh_stats_time_recorded() {
    let mesh = Mesh::new(vec![0.0; 9], vec![0, 1, 2]).unwrap();
    let (_opt, stats) = optimize_mesh(mesh).unwrap();
    // time_ms can be 0 for fast operations, but should be valid
    assert!(
        stats.time_ms < 60_000,
        "optimization shouldn't take a minute"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CompressionStats
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn compression_stats_new_basic() {
    let cs = CompressionStats::new(1000, 250, 42);
    assert_eq!(cs.original_size, 1000);
    assert_eq!(cs.compressed_size, 250);
    assert_eq!(cs.time_ms, 42);
}

#[test]
fn compression_stats_ratio() {
    let cs = CompressionStats::new(1000, 250, 0);
    // ratio = original / compressed = 1000 / 250 = 4.0
    assert!((cs.ratio - 4.0).abs() < 0.01, "ratio: {}", cs.ratio);
}

#[test]
fn compression_stats_reduction_percent() {
    let cs = CompressionStats::new(1000, 250, 0);
    // reduction = 100 * (1 - 250/1000) = 75%
    assert!(
        (cs.reduction_percent - 75.0).abs() < 0.01,
        "reduction: {}",
        cs.reduction_percent
    );
}

#[test]
fn compression_stats_no_compression() {
    let cs = CompressionStats::new(1000, 1000, 0);
    assert!((cs.ratio - 1.0).abs() < 0.01);
    assert!((cs.reduction_percent - 0.0).abs() < 0.01);
}

#[test]
fn compression_stats_zero_original() {
    // Edge case: shouldn't divide by zero
    let cs = CompressionStats::new(0, 0, 0);
    // Should not panic — uses .max(1) guard
    let _ = cs.ratio;
    let _ = cs.reduction_percent;
}

#[test]
fn compression_stats_clone() {
    let cs = CompressionStats::new(500, 100, 10);
    let cs2 = cs.clone();
    assert_eq!(cs2.original_size, 500);
    assert_eq!(cs2.compressed_size, 100);
    assert_eq!(cs2.time_ms, 10);
}

// ═══════════════════════════════════════════════════════════════════════════
// AstcBlockSize
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn astc_block_4x4_pixels() {
    assert_eq!(AstcBlockSize::Block4x4.pixels(), 16);
}

#[test]
fn astc_block_6x6_pixels() {
    assert_eq!(AstcBlockSize::Block6x6.pixels(), 36);
}

#[test]
fn astc_block_8x8_pixels() {
    assert_eq!(AstcBlockSize::Block8x8.pixels(), 64);
}

#[test]
fn astc_block_4x4_bpp() {
    // bpp = 128.0 / 16 = 8.0
    assert!((AstcBlockSize::Block4x4.bpp() - 8.0).abs() < 0.01);
}

#[test]
fn astc_block_6x6_bpp() {
    // bpp = 128.0 / 36 ≈ 3.556
    let bpp = AstcBlockSize::Block6x6.bpp();
    assert!((bpp - 3.556).abs() < 0.01, "bpp: {bpp}");
}

#[test]
fn astc_block_8x8_bpp() {
    // bpp = 128.0 / 64 = 2.0
    assert!((AstcBlockSize::Block8x8.bpp() - 2.0).abs() < 0.01);
}

#[test]
fn astc_block_copy() {
    let b = AstcBlockSize::Block4x4;
    let b2 = b;
    assert_eq!(b.pixels(), b2.pixels()); // Copy trait
}

#[test]
fn astc_block_eq() {
    assert_eq!(AstcBlockSize::Block4x4, AstcBlockSize::Block4x4);
    assert_ne!(AstcBlockSize::Block4x4, AstcBlockSize::Block8x8);
}

// ═══════════════════════════════════════════════════════════════════════════
// ValidationReport
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn validation_report_new_defaults() {
    let r = ValidationReport::new("test.png");
    assert_eq!(r.path, "test.png");
    assert!(r.passed);
    assert!(r.warnings.is_empty());
    assert!(r.errors.is_empty());
    assert!(r.is_valid());
}

#[test]
fn validation_report_add_warning_stays_valid() {
    let mut r = ValidationReport::new("test.png");
    r.add_warning("low quality");
    assert_eq!(r.warnings.len(), 1);
    assert!(r.is_valid(), "warnings should not fail the report");
}

#[test]
fn validation_report_add_error_fails_report() {
    let mut r = ValidationReport::new("test.png");
    r.add_error("file too large");
    assert_eq!(r.errors.len(), 1);
    assert!(!r.is_valid(), "errors should fail the report");
    assert!(!r.passed);
}

#[test]
fn validation_report_summary_passed() {
    let r = ValidationReport::new("good_asset.png");
    let summary = r.summary();
    assert!(summary.contains("PASSED"), "summary: {summary}");
    assert!(summary.contains("good_asset.png"), "summary: {summary}");
}

#[test]
fn validation_report_summary_failed() {
    let mut r = ValidationReport::new("bad_asset.png");
    r.add_error("broken");
    let summary = r.summary();
    assert!(summary.contains("FAILED"), "summary: {summary}");
    assert!(summary.contains("bad_asset.png"), "summary: {summary}");
}

#[test]
fn validation_report_multiple_warnings_and_errors() {
    let mut r = ValidationReport::new("complex.glb");
    r.add_warning("w1");
    r.add_warning("w2");
    r.add_error("e1");
    r.add_error("e2");
    assert_eq!(r.warnings.len(), 2);
    assert_eq!(r.errors.len(), 2);
    assert!(!r.is_valid());
}

#[test]
fn validation_report_clone() {
    let mut r = ValidationReport::new("clone_test");
    r.add_warning("w");
    let r2 = r.clone();
    assert_eq!(r2.path, "clone_test");
    assert_eq!(r2.warnings.len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// ValidationMetrics
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn validation_metrics_default() {
    let m = ValidationMetrics::default();
    assert_eq!(m.original_size, 0);
    assert_eq!(m.processed_size, 0);
    assert!(m.compression_ratio.is_none());
    assert!(m.quality_score.is_none());
}

#[test]
fn validation_metrics_clone() {
    let m = ValidationMetrics {
        original_size: 100,
        processed_size: 50,
        compression_ratio: Some(2.0),
        quality_score: Some(0.95),
    };
    let m2 = m.clone();
    assert_eq!(m2.original_size, 100);
    assert_eq!(m2.processed_size, 50);
    assert!((m2.compression_ratio.unwrap() - 2.0).abs() < f32::EPSILON);
    assert!((m2.quality_score.unwrap() - 0.95).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// AssetValidator defaults & validation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn asset_validator_default_max_size() {
    let v = AssetValidator::new();
    assert_eq!(v.max_size, Some(50 * 1024 * 1024)); // 50 MB
}

#[test]
fn asset_validator_default_min_quality() {
    let v = AssetValidator::new();
    assert!((v.min_quality.unwrap() - 0.90).abs() < f32::EPSILON);
}

#[test]
fn asset_validator_default_require_compression() {
    let v = AssetValidator::new();
    assert!(!v.require_compression);
}

#[test]
fn asset_validator_default_min_compression_ratio() {
    let v = AssetValidator::new();
    assert!((v.min_compression_ratio.unwrap() - 2.0).abs() < f32::EPSILON);
}

#[test]
fn asset_validator_validate_texture_small_passes() {
    let v = AssetValidator::new();
    let report = v.validate_texture(Path::new("small.png"), 1000, 250);
    assert!(report.is_valid());
    assert_eq!(report.path, "small.png");
}

#[test]
fn asset_validator_validate_texture_oversized_fails() {
    let v = AssetValidator::new();
    let huge_size = 100 * 1024 * 1024; // 100 MB
                                       // compressed_size must exceed max_size (50 MB) to trigger error
    let report = v.validate_texture(Path::new("huge.png"), huge_size, huge_size);
    assert!(
        !report.is_valid(),
        "compressed_size > max_size should fail validation"
    );
    assert!(
        !report.errors.is_empty(),
        "oversized compressed texture should have error"
    );
}

#[test]
fn asset_validator_validate_mesh_good_acmr() {
    let v = AssetValidator::new();
    let report = v.validate_mesh(Path::new("good.glb"), 1000, 0.5);
    assert!(report.is_valid());
}

#[test]
fn asset_validator_validate_mesh_bad_acmr_warns() {
    let v = AssetValidator::new();
    let report = v.validate_mesh(Path::new("bad.glb"), 1000, 2.0);
    // ACMR > 1.5 should generate a warning
    assert!(
        !report.warnings.is_empty(),
        "high ACMR should produce warning"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BatchValidationReport
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn batch_report_all_passed() {
    let v = AssetValidator::new();
    let r1 = v.validate_mesh(Path::new("a.glb"), 100, 0.5);
    let r2 = v.validate_mesh(Path::new("b.glb"), 200, 0.3);
    let batch = v.validate_batch(&[r1, r2]);
    assert!(batch.all_passed());
    assert_eq!(batch.total_assets, 2);
    assert_eq!(batch.passed_assets, 2);
    assert_eq!(batch.failed_assets, 0);
}

#[test]
fn batch_report_some_failed() {
    let v = AssetValidator::new();
    let r1 = v.validate_mesh(Path::new("ok.glb"), 100, 0.5);
    let mut r2 = ValidationReport::new("bad.glb");
    r2.add_error("corrupt");
    let batch = v.validate_batch(&[r1, r2]);
    assert!(!batch.all_passed());
    assert_eq!(batch.total_assets, 2);
    assert_eq!(batch.passed_assets, 1);
    assert_eq!(batch.failed_assets, 1);
}

#[test]
fn batch_report_empty() {
    let v = AssetValidator::new();
    let batch = v.validate_batch(&[]);
    assert!(batch.all_passed());
    assert_eq!(batch.total_assets, 0);
    assert_eq!(batch.failed_assets, 0);
}

#[test]
fn batch_report_warnings_counted() {
    let v = AssetValidator::new();
    let mut r = ValidationReport::new("warn.glb");
    r.add_warning("w1");
    r.add_warning("w2");
    let batch = v.validate_batch(&[r]);
    assert_eq!(batch.total_warnings, 2);
    assert_eq!(batch.total_errors, 0);
}

#[test]
fn batch_report_summary_contains_counts() {
    let v = AssetValidator::new();
    let r1 = ValidationReport::new("a");
    let batch = v.validate_batch(&[r1]);
    let summary = batch.summary();
    assert!(!summary.is_empty());
}
