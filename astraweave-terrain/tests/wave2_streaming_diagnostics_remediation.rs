//! Wave 2 Proactive Remediation: streaming_diagnostics.rs (76 mutants across shards 18-19)
//!
//! Golden-value arithmetic tests for HitchDetector, MemoryStats,
//! StreamingDiagnostics report generation — targeting operator mutations.

use astraweave_terrain::{
    ChunkId, ChunkLoadState, DiagnosticReport, FrameStats, HitchDetector, LodStats, MemoryStats,
    StreamingDiagnostics, StreamingStats,
};
use glam::Vec3;

// ============================================================================
// HitchDetector — arithmetic golden values
// ============================================================================

#[test]
fn hitch_detector_empty_average_is_zero() {
    let d = HitchDetector::new(100, 16.67);
    assert_eq!(d.average_frame_time(), 0.0);
}

#[test]
fn hitch_detector_empty_p99_is_zero() {
    let d = HitchDetector::new(100, 16.67);
    assert_eq!(d.p99_frame_time(), 0.0);
}

#[test]
fn hitch_detector_empty_hitch_rate_is_zero() {
    let d = HitchDetector::new(100, 16.67);
    assert_eq!(d.hitch_rate(), 0.0);
}

#[test]
fn hitch_detector_empty_hitch_count_is_zero() {
    let d = HitchDetector::new(100, 16.67);
    assert_eq!(d.hitch_count(), 0);
}

#[test]
fn hitch_detector_single_frame_average() {
    let mut d = HitchDetector::new(100, 16.67);
    d.record_frame(10.0);
    assert_eq!(d.average_frame_time(), 10.0);
}

#[test]
fn hitch_detector_two_frames_average() {
    let mut d = HitchDetector::new(100, 16.67);
    d.record_frame(10.0);
    d.record_frame(20.0);
    // average = 30/2 = 15
    assert_eq!(d.average_frame_time(), 15.0);
}

#[test]
fn hitch_detector_three_frames_average_golden() {
    let mut d = HitchDetector::new(100, 50.0);
    d.record_frame(12.0);
    d.record_frame(14.0);
    d.record_frame(16.0);
    // sum=42, count=3, avg=14.0
    assert_eq!(d.average_frame_time(), 14.0);
}

#[test]
fn hitch_detector_record_returns_true_for_hitch() {
    let mut d = HitchDetector::new(100, 16.67);
    // Below threshold
    assert!(!d.record_frame(16.0));
    // At threshold
    assert!(!d.record_frame(16.67));
    // Above threshold
    assert!(d.record_frame(16.68));
}

#[test]
fn hitch_detector_hitch_count_increments() {
    let mut d = HitchDetector::new(100, 10.0);
    d.record_frame(5.0); // not hitch
    d.record_frame(15.0); // hitch
    d.record_frame(5.0); // not hitch
    d.record_frame(20.0); // hitch
    assert_eq!(d.hitch_count(), 2);
}

#[test]
fn hitch_detector_hitch_rate_golden() {
    let mut d = HitchDetector::new(100, 10.0);
    // 8 normal, 2 hitches = 20% rate
    for _ in 0..8 {
        d.record_frame(5.0);
    }
    d.record_frame(15.0);
    d.record_frame(20.0);
    // rate = (2/10) * 100 = 20.0
    assert!((d.hitch_rate() - 20.0).abs() < 0.01);
}

#[test]
fn hitch_detector_hitch_rate_100_percent() {
    let mut d = HitchDetector::new(100, 1.0);
    for _ in 0..5 {
        d.record_frame(10.0); // all hitches
    }
    assert_eq!(d.hitch_rate(), 100.0);
}

#[test]
fn hitch_detector_p99_single_frame() {
    let mut d = HitchDetector::new(100, 50.0);
    d.record_frame(42.0);
    assert_eq!(d.p99_frame_time(), 42.0);
}

#[test]
fn hitch_detector_p99_100_frames_golden() {
    let mut d = HitchDetector::new(200, 100.0);
    // 99 low frames + 1 high frame
    for _ in 0..99 {
        d.record_frame(10.0);
    }
    d.record_frame(50.0);
    // sorted: [10.0 x99, 50.0 x1]
    // 0.99 * 100 = 99.0, ceil = 99, min(99, 99) = 99 → index 99 → 50.0
    assert_eq!(d.p99_frame_time(), 50.0);
}

#[test]
fn hitch_detector_p99_two_frames() {
    let mut d = HitchDetector::new(100, 100.0);
    d.record_frame(5.0);
    d.record_frame(25.0);
    // sorted: [5.0, 25.0]
    // 0.99 * 2 = 1.98, ceil = 2, min(2, 1) = 1 → index 1 → 25.0
    assert_eq!(d.p99_frame_time(), 25.0);
}

#[test]
fn hitch_detector_eviction_removes_old_hitch() {
    let mut d = HitchDetector::new(5, 10.0);
    d.record_frame(20.0); // hitch (1)
    assert_eq!(d.hitch_count(), 1);

    // Add 5 more normal frames → evicts the hitch
    for _ in 0..5 {
        d.record_frame(5.0);
    }
    assert_eq!(d.hitch_count(), 0);
}

#[test]
fn hitch_detector_eviction_keeps_non_hitch() {
    let mut d = HitchDetector::new(5, 10.0);
    d.record_frame(5.0); // not a hitch
    assert_eq!(d.hitch_count(), 0);

    // Fill up → evicts normal frame
    for _ in 0..5 {
        d.record_frame(5.0);
    }
    // hitch_count should remain 0 (no saturating_sub underflow)
    assert_eq!(d.hitch_count(), 0);
}

#[test]
fn hitch_detector_eviction_doesnt_underflow() {
    let mut d = HitchDetector::new(3, 10.0);
    d.record_frame(5.0);
    d.record_frame(5.0);
    d.record_frame(5.0);
    // hitch_count = 0, now a normal frame evicts oldest normal
    d.record_frame(5.0);
    assert_eq!(d.hitch_count(), 0); // Must not underflow
}

#[test]
fn hitch_detector_average_after_eviction() {
    let mut d = HitchDetector::new(3, 100.0);
    d.record_frame(10.0);
    d.record_frame(20.0);
    d.record_frame(30.0);
    // avg = 60/3 = 20.0
    assert_eq!(d.average_frame_time(), 20.0);

    d.record_frame(40.0); // evicts 10.0
                          // sum = 20+30+40 = 90, avg = 90/3 = 30.0
    assert_eq!(d.average_frame_time(), 30.0);
}

// ============================================================================
// MemoryStats — arithmetic golden values
// ============================================================================

#[test]
fn memory_stats_default_all_zero() {
    let s = MemoryStats::default();
    assert_eq!(s.total_bytes, 0);
    assert_eq!(s.bytes_per_chunk, 0);
    assert_eq!(s.chunk_count, 0);
    assert_eq!(s.peak_bytes, 0);
}

#[test]
fn memory_stats_update_calculates_total() {
    let mut s = MemoryStats::default();
    s.update(10, 1024);
    assert_eq!(s.total_bytes, 10 * 1024);
    assert_eq!(s.chunk_count, 10);
    assert_eq!(s.bytes_per_chunk, 1024);
}

#[test]
fn memory_stats_update_sets_peak() {
    let mut s = MemoryStats::default();
    s.update(10, 1024);
    assert_eq!(s.peak_bytes, 10 * 1024);
}

#[test]
fn memory_stats_peak_tracks_maximum() {
    let mut s = MemoryStats::default();
    s.update(100, 1000); // peak = 100000
    s.update(50, 1000); // peak stays 100000
    assert_eq!(s.peak_bytes, 100_000);
    assert_eq!(s.total_bytes, 50_000);

    s.update(200, 1000); // new peak = 200000
    assert_eq!(s.peak_bytes, 200_000);
}

#[test]
fn memory_stats_total_mb_golden() {
    let mut s = MemoryStats::default();
    s.update(1, 1_048_576); // 1 MB
    assert_eq!(s.total_mb(), 1.0);

    s.update(2, 1_048_576); // 2 MB
    assert_eq!(s.total_mb(), 2.0);
}

#[test]
fn memory_stats_total_mb_fractional() {
    let mut s = MemoryStats::default();
    s.update(1, 524_288); // 0.5 MB
    assert!((s.total_mb() - 0.5).abs() < 0.001);
}

#[test]
fn memory_stats_delta_from_peak_zero_peak() {
    let s = MemoryStats::default();
    // peak=0 → returns 0.0 to avoid div-by-zero
    assert_eq!(s.delta_from_peak_percent(), 0.0);
}

#[test]
fn memory_stats_delta_from_peak_at_peak() {
    let mut s = MemoryStats::default();
    s.update(100, 1000);
    // at peak: (100000/100000 - 1) * 100 = 0.0
    assert_eq!(s.delta_from_peak_percent(), 0.0);
}

#[test]
fn memory_stats_delta_from_peak_below() {
    let mut s = MemoryStats::default();
    s.update(100, 1000); // peak=100000
    s.update(50, 1000); // total=50000
                        // (50000/100000 - 1) * 100 = -50.0
    assert_eq!(s.delta_from_peak_percent(), -50.0);
}

#[test]
fn memory_stats_delta_from_peak_quarter() {
    let mut s = MemoryStats::default();
    s.update(100, 1000);
    s.update(25, 1000);
    // (25000/100000 - 1) * 100 = -75.0
    assert_eq!(s.delta_from_peak_percent(), -75.0);
}

// ============================================================================
// StreamingDiagnostics — report generation logic
// ============================================================================

#[test]
fn diagnostics_new_initial_state() {
    let diag = StreamingDiagnostics::new(16.67, 100);
    assert_eq!(diag.camera_pos(), Vec3::ZERO);
    assert!(diag.get_all_chunk_states().is_empty());
    assert_eq!(diag.hitch_detector().hitch_count(), 0);
    assert_eq!(diag.memory_stats().total_bytes, 0);
}

#[test]
fn diagnostics_chunk_states_loaded() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_chunk_states(&[ChunkId::new(0, 0), ChunkId::new(1, 0)], &[], &[]);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Loaded
    );
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(1, 0)),
        ChunkLoadState::Loaded
    );
}

#[test]
fn diagnostics_chunk_states_loading() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_chunk_states(&[], &[ChunkId::new(5, 5)], &[]);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(5, 5)),
        ChunkLoadState::Loading
    );
}

#[test]
fn diagnostics_chunk_states_pending() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_chunk_states(&[], &[], &[ChunkId::new(10, 10)]);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(10, 10)),
        ChunkLoadState::Pending
    );
}

#[test]
fn diagnostics_unknown_chunk_is_unloaded() {
    let diag = StreamingDiagnostics::new(16.67, 100);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(99, 99)),
        ChunkLoadState::Unloaded
    );
}

#[test]
fn diagnostics_update_clears_old_states() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_chunk_states(&[ChunkId::new(0, 0)], &[], &[]);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Loaded
    );

    // Update with different chunks — old one should be gone
    diag.update_chunk_states(&[ChunkId::new(1, 1)], &[], &[]);
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(0, 0)),
        ChunkLoadState::Unloaded
    );
    assert_eq!(
        diag.get_chunk_state(ChunkId::new(1, 1)),
        ChunkLoadState::Loaded
    );
}

#[test]
fn diagnostics_report_chunk_counts_correct() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_chunk_states(
        &[ChunkId::new(0, 0), ChunkId::new(1, 0), ChunkId::new(2, 0)],
        &[ChunkId::new(3, 0), ChunkId::new(4, 0)],
        &[ChunkId::new(5, 0)],
    );
    let report = diag.generate_report();
    assert_eq!(report.chunk_counts.loaded, 3);
    assert_eq!(report.chunk_counts.loading, 2);
    assert_eq!(report.chunk_counts.pending, 1);
}

#[test]
fn diagnostics_report_frame_stats_from_detector() {
    let mut diag = StreamingDiagnostics::new(10.0, 100);
    for _ in 0..5 {
        diag.record_frame(8.0);
    }
    diag.record_frame(15.0); // hitch

    let report = diag.generate_report();
    assert_eq!(report.frame_stats.hitch_count, 1);
    assert!(report.frame_stats.average_ms > 0.0);
    assert!(report.frame_stats.hitch_rate > 0.0);
}

#[test]
fn diagnostics_report_memory_from_stats() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_memory(25, 2048);

    let report = diag.generate_report();
    assert_eq!(report.memory.chunk_count, 25);
    assert_eq!(report.memory.bytes_per_chunk, 2048);
    assert_eq!(report.memory.total_bytes, 25 * 2048);
}

#[test]
fn diagnostics_report_lod_stats() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    diag.update_lod_stats(LodStats {
        total_chunks: 100,
        full_count: 40,
        half_count: 30,
        quarter_count: 20,
        skybox_count: 10,
        transitioning_count: 5,
    });

    let report = diag.generate_report();
    assert_eq!(report.lod.total_chunks, 100);
    assert_eq!(report.lod.full_count, 40);
    assert_eq!(report.lod.half_count, 30);
    assert_eq!(report.lod.quarter_count, 20);
    assert_eq!(report.lod.skybox_count, 10);
    assert_eq!(report.lod.transitioning_count, 5);
}

#[test]
fn diagnostics_camera_update() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    let pos = Vec3::new(100.0, 50.0, 200.0);
    diag.update_camera(pos);
    assert_eq!(diag.camera_pos(), pos);
}

#[test]
fn diagnostics_streaming_stats_passthrough() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    let stats = StreamingStats {
        loaded_chunk_count: 42,
        pending_load_count: 7,
        ..Default::default()
    };
    diag.update_streaming_stats(stats);
    assert_eq!(diag.streaming_stats().loaded_chunk_count, 42);
    assert_eq!(diag.streaming_stats().pending_load_count, 7);
}

// ============================================================================
// ChunkLoadState equality & discrimination
// ============================================================================

#[test]
fn chunk_load_state_all_variants_distinct() {
    let variants = [
        ChunkLoadState::Loaded,
        ChunkLoadState::Loading,
        ChunkLoadState::Pending,
        ChunkLoadState::Unloaded,
    ];
    for i in 0..variants.len() {
        for j in 0..variants.len() {
            if i == j {
                assert_eq!(variants[i], variants[j]);
            } else {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }
}

#[test]
fn diagnostics_duplicate_chunk_in_loaded_and_loading() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    let chunk = ChunkId::new(0, 0);
    // Same chunk in loaded and loading — loading inserted last, wins
    diag.update_chunk_states(&[chunk], &[chunk], &[]);
    assert_eq!(diag.get_chunk_state(chunk), ChunkLoadState::Loading);
}

#[test]
fn diagnostics_duplicate_chunk_in_all_three() {
    let mut diag = StreamingDiagnostics::new(16.67, 100);
    let chunk = ChunkId::new(0, 0);
    // Same chunk in all — pending inserted last, wins
    diag.update_chunk_states(&[chunk], &[chunk], &[chunk]);
    assert_eq!(diag.get_chunk_state(chunk), ChunkLoadState::Pending);
}

// ============================================================================
// Edge case: single-element history
// ============================================================================

#[test]
fn hitch_detector_history_size_1() {
    let mut d = HitchDetector::new(1, 10.0);
    d.record_frame(5.0);
    assert_eq!(d.average_frame_time(), 5.0);
    assert_eq!(d.hitch_count(), 0);

    d.record_frame(15.0); // evicts 5.0 (not hitch), adds 15.0 (hitch)
    assert_eq!(d.hitch_count(), 1);
    assert_eq!(d.average_frame_time(), 15.0);

    d.record_frame(5.0); // evicts 15.0 (hitch), adds 5.0 (not hitch)
    assert_eq!(d.hitch_count(), 0);
    assert_eq!(d.average_frame_time(), 5.0);
}
