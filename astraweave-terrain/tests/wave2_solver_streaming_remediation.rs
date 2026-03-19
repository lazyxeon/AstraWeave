//! Wave 2 Shard 18 — solver + streaming_diagnostics remediation
//!
//! Targets:
//!   solver.rs: 13 MISSED (private methods — tested indirectly through resolve_location)
//!   streaming_diagnostics.rs: 1 MISSED (> vs >= in HitchDetector::record_frame)

use astraweave_terrain::streaming_diagnostics::HitchDetector;

// ═══════════ HitchDetector boundary tests ══════════════════════
// Missed mutation: replace > with >= in record_frame
// The distinction matters: frame_time == threshold should NOT be a hitch

#[test]
fn hitch_detector_above_threshold_is_hitch() {
    let mut detector = HitchDetector::new(100, 16.0);
    let is_hitch = detector.record_frame(16.1); // slightly above
    assert!(is_hitch, "frame_time > threshold should be a hitch");
}

#[test]
fn hitch_detector_below_threshold_not_hitch() {
    let mut detector = HitchDetector::new(100, 16.0);
    let is_hitch = detector.record_frame(15.9); // slightly below
    assert!(!is_hitch, "frame_time < threshold should NOT be a hitch");
}

#[test]
fn hitch_detector_exactly_at_threshold_not_hitch() {
    // THIS is the critical test catching > vs >= mutation
    let mut detector = HitchDetector::new(100, 16.0);
    let is_hitch = detector.record_frame(16.0); // exactly at threshold
    assert!(
        !is_hitch,
        "frame_time == threshold should NOT be a hitch (uses > not >=)"
    );
}

#[test]
fn hitch_detector_exactly_at_threshold_hitch_count_zero() {
    let mut detector = HitchDetector::new(100, 16.0);
    detector.record_frame(16.0); // exactly at threshold — not a hitch
    assert_eq!(
        detector.hitch_count(),
        0,
        "recording exactly-at-threshold frame should not increment hitch_count"
    );
}

#[test]
fn hitch_detector_above_threshold_increments_hitch_count() {
    let mut detector = HitchDetector::new(100, 16.0);
    detector.record_frame(16.1);
    assert_eq!(detector.hitch_count(), 1);
    detector.record_frame(20.0);
    assert_eq!(detector.hitch_count(), 2);
    detector.record_frame(10.0);
    assert_eq!(detector.hitch_count(), 2); // not a hitch
}

#[test]
fn hitch_detector_average_frame_time() {
    let mut detector = HitchDetector::new(100, 16.0);
    detector.record_frame(10.0);
    detector.record_frame(20.0);
    let avg = detector.average_frame_time();
    assert!(
        (avg - 15.0).abs() < 0.01,
        "average of 10 and 20 should be 15, got {}",
        avg
    );
}

#[test]
fn hitch_detector_hitch_rate_calculation() {
    let mut detector = HitchDetector::new(100, 16.0);
    // 2 hitches out of 4 frames = 50%
    detector.record_frame(10.0); // no hitch
    detector.record_frame(20.0); // hitch
    detector.record_frame(5.0); // no hitch
    detector.record_frame(30.0); // hitch

    let rate = detector.hitch_rate();
    assert!(
        (rate - 50.0).abs() < 1.0,
        "2 hitches / 4 frames should be 50.0%, got {}",
        rate
    );
}

#[test]
fn hitch_detector_p99_frame_time() {
    let mut detector = HitchDetector::new(100, 16.0);
    for i in 0..100 {
        detector.record_frame(i as f32);
    }
    let p99 = detector.p99_frame_time();
    // P99 of 0..99 should be ~99 (or close to the 99th percentile)
    assert!(
        p99 > 90.0,
        "p99 should be above 90 for frames 0..99, got {}",
        p99
    );
}

#[test]
fn hitch_detector_history_eviction() {
    // max_history=3, record 4 frames → oldest should be evicted
    let mut detector = HitchDetector::new(3, 16.0);
    detector.record_frame(50.0); // hitch (will be evicted)
    detector.record_frame(10.0);
    detector.record_frame(10.0);
    detector.record_frame(10.0); // 4th frame → evicts 50.0

    // hitch_count should be decremented when hitchy frame is evicted
    assert_eq!(
        detector.hitch_count(),
        0,
        "hitch count should be 0 after hitchy frame is evicted"
    );
}

#[test]
fn hitch_detector_history_eviction_preserves_non_hitch_count() {
    let mut detector = HitchDetector::new(3, 16.0);
    detector.record_frame(5.0); // no hitch (will be evicted)
    detector.record_frame(20.0); // hitch
    detector.record_frame(10.0);
    detector.record_frame(10.0); // evicts 5.0 (non-hitch)

    // hitch_count should stay at 1 (20.0 still in window)
    assert_eq!(
        detector.hitch_count(),
        1,
        "evicting non-hitch frame should not decrement count"
    );
}
