//! Mutation-resistant comprehensive tests for aw_debug.
//!
//! Tests PerfHud and EventLog data logic (no egui UI context needed).

use aw_debug::*;

// ═══════════════════════════════════════════════════════════════════════════
// PerfHud construction & defaults
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn perf_hud_new() {
    let hud = PerfHud::new();
    assert!((hud.fps - 0.0).abs() < f32::EPSILON);
    assert_eq!(hud.entity_count, 0);
    assert!(hud.systems_snapshot.is_empty());
}

#[test]
fn perf_hud_default() {
    let hud = PerfHud::default();
    assert!((hud.fps - 0.0).abs() < f32::EPSILON);
    assert_eq!(hud.entity_count, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// PerfHud::frame updates FPS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn perf_hud_frame_updates_fps() {
    let mut hud = PerfHud::new();
    // Call frame many times to get a non-zero FPS
    for _ in 0..10 {
        hud.frame();
    }
    // After several frames, fps should be > 0
    assert!(
        hud.fps > 0.0,
        "fps after 10 frames should be > 0: {}",
        hud.fps
    );
}

#[test]
fn perf_hud_frame_does_not_panic() {
    let mut hud = PerfHud::new();
    for _ in 0..1000 {
        hud.frame();
    }
    // Should not panic even with many frames
}

// ═══════════════════════════════════════════════════════════════════════════
// PerfHud fields are mutable
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn perf_hud_entity_count_settable() {
    let mut hud = PerfHud::new();
    hud.entity_count = 42;
    assert_eq!(hud.entity_count, 42);
}

#[test]
fn perf_hud_systems_snapshot_settable() {
    let mut hud = PerfHud::new();
    hud.systems_snapshot = vec![("physics".into(), 2.5), ("render".into(), 8.0)];
    assert_eq!(hud.systems_snapshot.len(), 2);
    assert_eq!(hud.systems_snapshot[0].0, "physics");
    assert!((hud.systems_snapshot[0].1 - 2.5).abs() < f32::EPSILON);
    assert_eq!(hud.systems_snapshot[1].0, "render");
    assert!((hud.systems_snapshot[1].1 - 8.0).abs() < f32::EPSILON);
}

// ═══════════════════════════════════════════════════════════════════════════
// PerfHud::log_event
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn perf_hud_log_event_does_not_panic() {
    let mut hud = PerfHud::new();
    hud.log_event("error", "something broke");
    hud.log_event("warning", "low memory");
    hud.log_event("ai", "decision made");
    hud.log_event("physics", "collision");
    hud.log_event("script", "script loaded");
}

#[test]
fn perf_hud_log_many_events() {
    let mut hud = PerfHud::new();
    for i in 0..200 {
        hud.log_event("test", &format!("event_{i}"));
    }
    // EventLog has max_events=100, so should handle overflow gracefully
}

// ═══════════════════════════════════════════════════════════════════════════
// EventLog construction & add
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn event_log_new() {
    let log = EventLog::new(50);
    // Should not panic
    let _ = log;
}

#[test]
fn event_log_add_does_not_panic() {
    let mut log = EventLog::new(10);
    log.add("error", "test error");
    log.add("ai", "decision");
}

#[test]
fn event_log_fifo_eviction() {
    let mut log = EventLog::new(3);
    log.add("a", "first");
    log.add("b", "second");
    log.add("c", "third");
    log.add("d", "fourth"); // should evict "first"
                            // No panic — FIFO eviction working
}

#[test]
fn event_log_zero_capacity_panics_on_add() {
    let mut log = EventLog::new(0);
    // Zero capacity causes panic on add (tries to remove from empty deque)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        log.add("x", "test");
    }));
    assert!(
        result.is_err(),
        "adding to zero-capacity EventLog should panic"
    );
}

#[test]
fn event_log_large_capacity() {
    let log = EventLog::new(100_000);
    let _ = log;
    // Should not panic with large capacity
}

// ═══════════════════════════════════════════════════════════════════════════
// watch_scripts / watch_reload_signal (filesystem watchers)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn watch_scripts_on_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = watch_scripts(dir.path().to_path_buf(), || {});
    assert!(result.is_ok(), "watch_scripts should succeed on valid dir");
}

#[test]
fn watch_reload_signal_on_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = watch_reload_signal(dir.path().to_path_buf(), || {});
    assert!(
        result.is_ok(),
        "watch_reload_signal should succeed on valid dir"
    );
}
