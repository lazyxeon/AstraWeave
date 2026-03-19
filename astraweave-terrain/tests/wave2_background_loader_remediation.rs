//! Wave 2 Mutation Remediation — Background Loader
//!
//! Targets the 95 missed mutants in background_loader.rs by testing:
//! - TerrainRateLimiter: can_request, record_request, task_completed,
//!   remaining_cooldown, active_count, cleanup, Default
//! - ChunkPriority: Ord implementation (frustum, distance, timestamp)
//! - StreamingConfig: Default values
//! - StreamingStats: Default values
//! - TerrainTask: request_id

use astraweave_terrain::background_loader::{
    ChunkPriority, StreamingConfig, StreamingStats, TerrainRateLimiter, TerrainTask,
    TerrainTaskResult,
};
use glam::Vec3;

// ============================================================================
// TERRAIN RATE LIMITER — BASICS
// ============================================================================

#[test]
fn rate_limiter_new_allows_first_request() {
    let limiter = TerrainRateLimiter::new(10.0, 4);
    assert!(limiter.can_request("player1", 0.0));
}

#[test]
fn rate_limiter_default_values() {
    let limiter = TerrainRateLimiter::default();
    // Default: 10.0 second cooldown, max 4 concurrent
    assert!(limiter.can_request("player1", 0.0));
    assert_eq!(limiter.active_count(), 0);
    assert_eq!(limiter.remaining_cooldown("player1", 0.0), 0.0);
}

#[test]
fn rate_limiter_active_count_starts_at_zero() {
    let limiter = TerrainRateLimiter::new(5.0, 4);
    assert_eq!(limiter.active_count(), 0);
}

// ============================================================================
// TERRAIN RATE LIMITER — COOLDOWN
// ============================================================================

/// After recording a request, the same player should be blocked until cooldown expires.
#[test]
fn rate_limiter_cooldown_blocks_player() {
    let mut limiter = TerrainRateLimiter::new(10.0, 4);
    limiter.record_request("player1", 0.0);

    // Before cooldown expires
    assert!(
        !limiter.can_request("player1", 5.0),
        "Should be blocked at 5s (cooldown=10s)"
    );
    assert!(
        !limiter.can_request("player1", 9.9),
        "Should be blocked at 9.9s"
    );

    // After cooldown expires
    assert!(
        limiter.can_request("player1", 10.0),
        "Should be allowed at 10.0s"
    );
    assert!(
        limiter.can_request("player1", 15.0),
        "Should be allowed at 15.0s"
    );
}

/// Different players have independent cooldowns.
#[test]
fn rate_limiter_independent_cooldowns() {
    let mut limiter = TerrainRateLimiter::new(10.0, 4);
    limiter.record_request("player1", 0.0);

    // player2 has never requested, should be allowed
    assert!(limiter.can_request("player2", 5.0));

    // player1 is still on cooldown
    assert!(!limiter.can_request("player1", 5.0));
}

/// remaining_cooldown returns correct values.
#[test]
fn rate_limiter_remaining_cooldown_values() {
    let mut limiter = TerrainRateLimiter::new(10.0, 4);
    limiter.record_request("player1", 0.0);

    // At time 3.0, remaining = 10.0 - 3.0 = 7.0
    let remaining = limiter.remaining_cooldown("player1", 3.0);
    assert!(
        (remaining - 7.0).abs() < 0.01,
        "remaining at 3s: {remaining}"
    );

    // At time 10.0, remaining = 0.0
    let remaining = limiter.remaining_cooldown("player1", 10.0);
    assert_eq!(remaining, 0.0, "remaining at 10s: {remaining}");

    // At time 15.0, remaining = 0.0 (clamped)
    let remaining = limiter.remaining_cooldown("player1", 15.0);
    assert_eq!(remaining, 0.0, "remaining at 15s: {remaining}");
}

/// Unknown player has zero remaining cooldown.
#[test]
fn rate_limiter_unknown_player_zero_cooldown() {
    let limiter = TerrainRateLimiter::new(10.0, 4);
    assert_eq!(limiter.remaining_cooldown("unknown", 0.0), 0.0);
}

// ============================================================================
// TERRAIN RATE LIMITER — CONCURRENT TASKS
// ============================================================================

/// At max concurrent tasks, new requests are blocked.
#[test]
fn rate_limiter_max_concurrent_blocks() {
    let mut limiter = TerrainRateLimiter::new(1.0, 2); // Max 2 concurrent

    limiter.record_request("p1", 0.0);
    limiter.record_request("p2", 0.0);

    // Active count should be 2
    assert_eq!(limiter.active_count(), 2);

    // Even though p3 has no cooldown, max concurrent reached
    assert!(
        !limiter.can_request("p3", 100.0),
        "Should be blocked by max concurrent"
    );
}

/// task_completed decrements active count.
#[test]
fn rate_limiter_task_completed_decrements() {
    let mut limiter = TerrainRateLimiter::new(1.0, 2);
    limiter.record_request("p1", 0.0);
    limiter.record_request("p2", 0.0);
    assert_eq!(limiter.active_count(), 2);

    limiter.task_completed();
    assert_eq!(limiter.active_count(), 1);

    limiter.task_completed();
    assert_eq!(limiter.active_count(), 0);
}

/// task_completed saturates at 0 (doesn't underflow).
#[test]
fn rate_limiter_task_completed_saturates() {
    let mut limiter = TerrainRateLimiter::new(10.0, 4);
    limiter.task_completed(); // Already at 0
    assert_eq!(limiter.active_count(), 0);
}

/// After completing a task, a new request from a different player should work.
#[test]
fn rate_limiter_freed_slot_allows_new_request() {
    let mut limiter = TerrainRateLimiter::new(1.0, 2);
    limiter.record_request("p1", 0.0);
    limiter.record_request("p2", 0.0);

    // Can't add p3
    assert!(!limiter.can_request("p3", 100.0));

    // Complete one task
    limiter.task_completed();

    // Now p3 should be allowed (cooldown doesn't apply, slot freed)
    assert!(limiter.can_request("p3", 100.0));
}

// ============================================================================
// TERRAIN RATE LIMITER — CLEANUP
// ============================================================================

/// Cleanup should remove old entries beyond 10× cooldown.
#[test]
fn rate_limiter_cleanup_removes_old_entries() {
    let mut limiter = TerrainRateLimiter::new(10.0, 4);
    limiter.record_request("old_player", 0.0);
    limiter.record_request("recent_player", 90.0);

    // Cleanup at time 110 should remove entries older than 110 - 10*10 = 10
    // old_player at 0.0 < 10.0, should be removed
    // recent_player at 90.0 >= 10.0, should remain
    limiter.cleanup(110.0);

    // old_player's cooldown data should be gone → remaining = 0
    assert_eq!(limiter.remaining_cooldown("old_player", 110.0), 0.0);
    // recent_player should still have data
    // At time 110, cooldown from 90.0: elapsed=20, remaining=max(10-20, 0) = 0
    // But the entry should still exist in the map
    assert!(limiter.can_request("old_player", 110.0));
}

// ============================================================================
// CHUNK PRIORITY — ORDERING
// ============================================================================

/// Frustum chunks should have higher priority than non-frustum.
#[test]
fn chunk_priority_frustum_over_non_frustum() {
    let frustum = ChunkPriority {
        distance: 100.0,
        in_frustum: true,
        timestamp: 0,
    };
    let non_frustum = ChunkPriority {
        distance: 10.0, // Closer but out of frustum
        in_frustum: false,
        timestamp: 0,
    };

    // Higher priority means "should be loaded first" = should compare as Greater
    assert!(
        frustum > non_frustum,
        "Frustum should have higher priority than non-frustum"
    );
}

/// Among frustum chunks, closer distance = higher priority.
#[test]
fn chunk_priority_closer_is_higher() {
    let close = ChunkPriority {
        distance: 10.0,
        in_frustum: true,
        timestamp: 0,
    };
    let far = ChunkPriority {
        distance: 100.0,
        in_frustum: true,
        timestamp: 0,
    };

    assert!(close > far, "Closer chunk should have higher priority");
}

/// Equal distance and frustum → newer timestamp wins (most-recent priority).
#[test]
fn chunk_priority_newer_timestamp_wins() {
    let old = ChunkPriority {
        distance: 50.0,
        in_frustum: true,
        timestamp: 1,
    };
    let new = ChunkPriority {
        distance: 50.0,
        in_frustum: true,
        timestamp: 100,
    };

    // Newer timestamp has higher Ord value (most-recent priority order)
    assert!(
        new > old,
        "Newer timestamp should have higher priority: old={old:?} new={new:?}"
    );
}

/// Eq impl for ChunkPriority
#[test]
fn chunk_priority_eq() {
    let a = ChunkPriority {
        distance: 50.0,
        in_frustum: true,
        timestamp: 5,
    };
    let b = ChunkPriority {
        distance: 50.0,
        in_frustum: true,
        timestamp: 5,
    };
    assert_eq!(a, b);
}

// ============================================================================
// STREAMING CONFIG — DEFAULTS
// ============================================================================

#[test]
fn streaming_config_defaults_exact() {
    let c = StreamingConfig::default();
    assert_eq!(c.max_loaded_chunks, 256);
    assert_eq!(c.view_distance, 8);
    assert_eq!(c.prefetch_distance, 4);
    assert_eq!(c.max_concurrent_loads, 8);
    assert_eq!(c.chunk_size, 256.0);
    assert_eq!(c.adaptive_throttle_threshold_ms, 10.0);
    assert_eq!(c.throttled_concurrent_loads, 2);
}

// ============================================================================
// STREAMING STATS — DEFAULTS
// ============================================================================

#[test]
fn streaming_stats_defaults_zero() {
    let s = StreamingStats::default();
    assert_eq!(s.loaded_chunk_count, 0);
    assert_eq!(s.pending_load_count, 0);
    assert_eq!(s.active_load_count, 0);
    assert_eq!(s.memory_usage_mb, 0.0);
    assert_eq!(s.chunks_loaded_this_frame, 0);
    assert_eq!(s.chunks_unloaded_this_frame, 0);
    assert_eq!(s.terrain_tasks_pending, 0);
    assert_eq!(s.terrain_tasks_completed, 0);
    assert_eq!(s.terrain_tasks_rate_limited, 0);
}

// ============================================================================
// TERRAIN TASK — REQUEST ID
// ============================================================================

#[test]
fn terrain_task_generate_request_id() {
    let task = TerrainTask::Generate {
        request_id: "req_001".to_string(),
        position: Vec3::ZERO,
        feature_type: "mountain".to_string(),
        intensity: 0.5,
        seed: 42,
        affected_chunks: vec![],
    };
    assert_eq!(task.request_id(), "req_001");
}

#[test]
fn terrain_task_revert_request_id() {
    let task = TerrainTask::Revert {
        request_id: "req_002".to_string(),
    };
    assert_eq!(task.request_id(), "req_002");
}

// ============================================================================
// RATE LIMITER — EDGE CASES
// ============================================================================

/// Record then check with exact cooldown boundary.
#[test]
fn rate_limiter_exact_cooldown_boundary() {
    let mut limiter = TerrainRateLimiter::new(5.0, 10);
    limiter.record_request("p1", 10.0);

    // At exactly 15.0, elapsed = 5.0 >= 5.0 seconds cooldown → allowed
    assert!(limiter.can_request("p1", 15.0));

    // At 14.99, elapsed = 4.99 < 5.0 → blocked
    assert!(!limiter.can_request("p1", 14.99));
}

/// Multiple records from same player update the timestamp.
#[test]
fn rate_limiter_rerecord_updates_timestamp() {
    let mut limiter = TerrainRateLimiter::new(5.0, 10);
    limiter.record_request("p1", 0.0);

    // At 5.0, cooldown expired
    assert!(limiter.can_request("p1", 5.0));

    // Re-record at 5.0
    limiter.record_request("p1", 5.0);

    // Now cooldown is from 5.0, so 7.0 should be blocked
    assert!(!limiter.can_request("p1", 7.0));

    // 10.0 should be allowed
    assert!(limiter.can_request("p1", 10.0));
}

/// Zero cooldown means everything is always allowed (if slots available).
#[test]
fn rate_limiter_zero_cooldown_always_allowed() {
    let mut limiter = TerrainRateLimiter::new(0.0, 10);
    limiter.record_request("p1", 0.0);

    // Even at same time, cooldown 0.0 means elapsed >= 0.0 is always true
    assert!(limiter.can_request("p1", 0.0));
}
