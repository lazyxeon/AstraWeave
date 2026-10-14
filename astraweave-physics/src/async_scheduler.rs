// async_scheduler.rs - Async Physics Pipeline with Rayon Parallelization
//
// This module implements a 3-stage parallel physics pipeline:
// 1. Broad-Phase: Coarse collision detection (parallel AABB checks)
// 2. Narrow-Phase: Fine collision resolution (parallel contact generation)
// 3. Integration: Apply forces and update positions (parallel per-island)
//
// Key Design:
// - Uses Rayon for deterministic parallel iteration
// - Maintains strict ordering between stages (barriers)
// - Exposes telemetry for performance profiling
// - Feature-gated behind `async-physics` feature flag

use std::time::{Duration, Instant};

/// Performance telemetry for a single physics step
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PhysicsStepProfile {
    /// Total time for complete physics step
    pub total_duration: Duration,

    /// Time spent in broad-phase collision detection
    pub broad_phase_duration: Duration,

    /// Time spent in narrow-phase collision resolution
    pub narrow_phase_duration: Duration,

    /// Time spent in integration (forces, positions)
    pub integration_duration: Duration,

    /// Number of active rigid bodies
    pub active_body_count: usize,

    /// Number of collision pairs detected
    pub collision_pair_count: usize,

    /// Number of constraint solver iterations
    pub solver_iterations: usize,
}

impl PhysicsStepProfile {
    /// Create a new profile with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate percentage of time spent in broad-phase
    pub fn broad_phase_percent(&self) -> f32 {
        if self.total_duration.as_nanos() == 0 {
            return 0.0;
        }
        (self.broad_phase_duration.as_nanos() as f32 / self.total_duration.as_nanos() as f32)
            * 100.0
    }

    /// Calculate percentage of time spent in narrow-phase
    pub fn narrow_phase_percent(&self) -> f32 {
        if self.total_duration.as_nanos() == 0 {
            return 0.0;
        }
        (self.narrow_phase_duration.as_nanos() as f32 / self.total_duration.as_nanos() as f32)
            * 100.0
    }

    /// Calculate percentage of time spent in integration
    pub fn integration_percent(&self) -> f32 {
        if self.total_duration.as_nanos() == 0 {
            return 0.0;
        }
        (self.integration_duration.as_nanos() as f32 / self.total_duration.as_nanos() as f32)
            * 100.0
    }
}

/// Async physics scheduler that coordinates parallel physics simulation
#[cfg(feature = "async-physics")]
pub struct AsyncPhysicsScheduler {
    /// Number of Rayon threads to use (0 = auto-detect)
    pub thread_count: usize,

    /// Last step profile (for telemetry)
    pub last_profile: PhysicsStepProfile,

    /// Enable/disable profiling (small overhead)
    pub enable_profiling: bool,
}

#[cfg(feature = "async-physics")]
impl Default for AsyncPhysicsScheduler {
    fn default() -> Self {
        Self {
            thread_count: 0, // Auto-detect (use Rayon's default)
            last_profile: PhysicsStepProfile::default(),
            enable_profiling: true,
        }
    }
}

#[cfg(feature = "async-physics")]
impl AsyncPhysicsScheduler {
    /// Create a new async scheduler
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scheduler with specific thread count
    pub fn with_threads(thread_count: usize) -> Self {
        Self {
            thread_count,
            ..Default::default()
        }
    }

    /// Get the last step profile (for telemetry/dashboard)
    pub fn get_last_profile(&self) -> PhysicsStepProfile {
        self.last_profile
    }

    /// Record telemetry for a completed physics step
    /// Called by PhysicsWorld::step() when async is enabled
    pub fn record_step_telemetry(&mut self, total_duration: Duration) {
        if !self.enable_profiling {
            return;
        }

        // Update profile with actual timing
        // Note: Rapier3D handles internal parallelization, so we just record total time
        self.last_profile.total_duration = total_duration;

        // For now, we don't break down stages since Rapier3D's parallelization
        // is internal. Future: add custom instrumentation hooks.
        self.last_profile.integration_duration = total_duration;
        self.last_profile.broad_phase_duration = Duration::ZERO;
        self.last_profile.narrow_phase_duration = Duration::ZERO;
    }

    /// Execute a physics step with timing wrapper
    ///
    /// Wraps an arbitrary physics step function with profiling.
    /// Use [`step_parallel_staged`] for the full 3-stage pipeline.
    pub fn step_parallel<F>(&mut self, mut step_fn: F) -> PhysicsStepProfile
    where
        F: FnMut() -> PhysicsStepProfile,
    {
        let start = Instant::now();
        let profile = step_fn();
        let total_duration = start.elapsed();

        let final_profile = PhysicsStepProfile {
            total_duration,
            ..profile
        };

        if self.enable_profiling {
            self.last_profile = final_profile;
        }

        final_profile
    }

    /// Execute a physics step with a 3-stage parallel pipeline
    ///
    /// Runs broad-phase, narrow-phase, and integration stages with
    /// barrier synchronization and per-stage profiling. Each stage's
    /// output feeds into the next, enforcing a strict execution order.
    ///
    /// # Pipeline Stages
    ///
    /// 1. **Broad-Phase**: Coarse collision detection (AABB pair checks).
    ///    Produces candidate collision pairs for narrow-phase.
    /// 2. **Narrow-Phase**: Fine collision resolution (contact generation).
    ///    Consumes broad-phase output, produces contact manifolds.
    /// 3. **Integration**: Force application, constraint solving, and
    ///    position updates. Consumes narrow-phase output.
    ///
    /// Each stage is separated by an implicit barrier — the next stage
    /// starts only after the previous completes. Within each stage,
    /// work can be parallelized using rayon (see [`parallel`] module).
    ///
    /// # Type Parameters
    /// * `BroadOut` - Data produced by broad-phase for narrow-phase
    /// * `NarrowOut` - Data produced by narrow-phase for integration
    ///
    /// # Returns
    /// `PhysicsStepProfile` with per-stage timing breakdown
    ///
    /// # Example
    /// ```ignore
    /// use astraweave_physics::async_scheduler::AsyncPhysicsScheduler;
    ///
    /// let mut scheduler = AsyncPhysicsScheduler::new();
    /// let profile = scheduler.step_parallel_staged(
    ///     || {
    ///         // Broad-phase: detect candidate collision pairs
    ///         let pairs = detect_aabb_overlaps(&bodies);
    ///         pairs
    ///     },
    ///     |pairs| {
    ///         // Narrow-phase: generate contacts for each pair
    ///         let contacts = generate_contacts(&pairs);
    ///         contacts
    ///     },
    ///     |contacts| {
    ///         // Integration: solve constraints and update positions
    ///         solve_and_integrate(&contacts, dt);
    ///     },
    /// );
    /// assert!(profile.broad_phase_duration > std::time::Duration::ZERO);
    /// ```
    pub fn step_parallel_staged<B, N, I, BroadOut, NarrowOut>(
        &mut self,
        broad_phase_fn: B,
        narrow_phase_fn: N,
        integration_fn: I,
    ) -> PhysicsStepProfile
    where
        B: FnOnce() -> BroadOut,
        N: FnOnce(BroadOut) -> NarrowOut,
        I: FnOnce(NarrowOut),
    {
        let start = Instant::now();

        // Stage 1: Broad-Phase (AABB overlap detection)
        let bp_start = Instant::now();
        let broad_out = broad_phase_fn();
        let broad_phase_duration = bp_start.elapsed();

        // Barrier: broad-phase complete → narrow-phase begins

        // Stage 2: Narrow-Phase (contact generation)
        let np_start = Instant::now();
        let narrow_out = narrow_phase_fn(broad_out);
        let narrow_phase_duration = np_start.elapsed();

        // Barrier: narrow-phase complete → integration begins

        // Stage 3: Integration (solver + position updates)
        let int_start = Instant::now();
        integration_fn(narrow_out);
        let integration_duration = int_start.elapsed();

        let total_duration = start.elapsed();

        let final_profile = PhysicsStepProfile {
            total_duration,
            broad_phase_duration,
            narrow_phase_duration,
            integration_duration,
            ..Default::default()
        };

        if self.enable_profiling {
            self.last_profile = final_profile;
        }

        final_profile
    }

    /// Execute a 3-stage pipeline with statistics collection
    ///
    /// Same as [`step_parallel_staged`] but also records body count,
    /// collision pair count, and solver iterations in the profile.
    pub fn step_parallel_staged_with_stats<B, N, I, BroadOut, NarrowOut>(
        &mut self,
        broad_phase_fn: B,
        narrow_phase_fn: N,
        integration_fn: I,
        active_body_count: usize,
        collision_pair_count: usize,
        solver_iterations: usize,
    ) -> PhysicsStepProfile
    where
        B: FnOnce() -> BroadOut,
        N: FnOnce(BroadOut) -> NarrowOut,
        I: FnOnce(NarrowOut),
    {
        let mut profile =
            self.step_parallel_staged(broad_phase_fn, narrow_phase_fn, integration_fn);
        profile.active_body_count = active_body_count;
        profile.collision_pair_count = collision_pair_count;
        profile.solver_iterations = solver_iterations;

        if self.enable_profiling {
            self.last_profile = profile;
        }

        profile
    }

    /// Export telemetry to JSON file (for benchmark dashboard)
    #[cfg(feature = "serde")]
    pub fn export_telemetry(&self, path: &std::path::Path) -> anyhow::Result<()> {
        use std::fs;
        use std::io::Write;

        let json = serde_json::to_string_pretty(&self.last_profile)?;
        let mut file = fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

/// Parallel iterator helpers for deterministic physics
#[cfg(feature = "async-physics")]
pub mod parallel {
    use rayon::prelude::*;

    /// Process rigid bodies in parallel (deterministic order)
    ///
    /// This helper ensures:
    /// - Bodies are processed in handle order (determinism)
    /// - Work is split across available threads
    /// - Results are collected in original order
    pub fn par_process_bodies<T, F>(bodies: &[T], f: F) -> Vec<T>
    where
        T: Send + Sync + Clone,
        F: Fn(&T) -> T + Send + Sync,
    {
        bodies.par_iter().map(f).collect()
    }

    /// Process collision pairs in parallel
    ///
    /// Collision pairs have no ordering dependency, so can be processed freely
    pub fn par_process_collision_pairs<T, F>(pairs: &[T], f: F)
    where
        T: Send + Sync,
        F: Fn(&T) + Send + Sync,
    {
        pairs.par_iter().for_each(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // PHYSICS STEP PROFILE TESTS
    // ============================================================================

    #[test]
    fn profile_percentages_sum_to_100() {
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_millis(10),
            broad_phase_duration: Duration::from_millis(4),
            narrow_phase_duration: Duration::from_millis(3),
            integration_duration: Duration::from_millis(3),
            active_body_count: 100,
            collision_pair_count: 50,
            solver_iterations: 4,
        };

        let total_percent = profile.broad_phase_percent()
            + profile.narrow_phase_percent()
            + profile.integration_percent();

        // Should sum to 100% (within floating point error)
        assert!((total_percent - 100.0).abs() < 1.0);
    }

    #[test]
    fn profile_default_values() {
        let profile = PhysicsStepProfile::default();

        assert_eq!(profile.total_duration, Duration::ZERO);
        assert_eq!(profile.broad_phase_duration, Duration::ZERO);
        assert_eq!(profile.narrow_phase_duration, Duration::ZERO);
        assert_eq!(profile.integration_duration, Duration::ZERO);
        assert_eq!(profile.active_body_count, 0);
        assert_eq!(profile.collision_pair_count, 0);
        assert_eq!(profile.solver_iterations, 0);
    }

    #[test]
    fn profile_new_equals_default() {
        let profile_new = PhysicsStepProfile::new();
        let profile_default = PhysicsStepProfile::default();

        assert_eq!(profile_new.total_duration, profile_default.total_duration);
        assert_eq!(
            profile_new.active_body_count,
            profile_default.active_body_count
        );
    }

    #[test]
    fn profile_percentages_zero_total_duration() {
        let profile = PhysicsStepProfile {
            total_duration: Duration::ZERO,
            broad_phase_duration: Duration::from_millis(4),
            narrow_phase_duration: Duration::from_millis(3),
            integration_duration: Duration::from_millis(3),
            active_body_count: 100,
            collision_pair_count: 50,
            solver_iterations: 4,
        };

        // Should not panic on divide by zero
        assert_eq!(profile.broad_phase_percent(), 0.0);
        assert_eq!(profile.narrow_phase_percent(), 0.0);
        assert_eq!(profile.integration_percent(), 0.0);
    }

    #[test]
    fn profile_percentages_single_phase_dominance() {
        // All time in integration
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_millis(10),
            broad_phase_duration: Duration::ZERO,
            narrow_phase_duration: Duration::ZERO,
            integration_duration: Duration::from_millis(10),
            active_body_count: 100,
            collision_pair_count: 50,
            solver_iterations: 4,
        };

        assert!((profile.integration_percent() - 100.0).abs() < 0.01);
        assert_eq!(profile.broad_phase_percent(), 0.0);
        assert_eq!(profile.narrow_phase_percent(), 0.0);
    }

    #[test]
    fn profile_very_small_durations() {
        // Nanosecond precision
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_nanos(1000),
            broad_phase_duration: Duration::from_nanos(333),
            narrow_phase_duration: Duration::from_nanos(333),
            integration_duration: Duration::from_nanos(334),
            active_body_count: 10,
            collision_pair_count: 5,
            solver_iterations: 1,
        };

        let total_percent = profile.broad_phase_percent()
            + profile.narrow_phase_percent()
            + profile.integration_percent();

        // Should sum close to 100%
        assert!(
            (total_percent - 100.0).abs() < 1.0,
            "Got {}% total",
            total_percent
        );
    }

    #[test]
    fn profile_very_large_durations() {
        // Hours of physics simulation (stress test)
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_secs(3600),
            broad_phase_duration: Duration::from_secs(1200),
            narrow_phase_duration: Duration::from_secs(1200),
            integration_duration: Duration::from_secs(1200),
            active_body_count: 1_000_000,
            collision_pair_count: 5_000_000,
            solver_iterations: 10,
        };

        let total_percent = profile.broad_phase_percent()
            + profile.narrow_phase_percent()
            + profile.integration_percent();

        assert!((total_percent - 100.0).abs() < 1.0);
    }

    #[test]
    fn profile_clone() {
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_millis(10),
            broad_phase_duration: Duration::from_millis(4),
            narrow_phase_duration: Duration::from_millis(3),
            integration_duration: Duration::from_millis(3),
            active_body_count: 100,
            collision_pair_count: 50,
            solver_iterations: 4,
        };

        let cloned = profile;
        assert_eq!(cloned.total_duration, profile.total_duration);
        assert_eq!(cloned.active_body_count, profile.active_body_count);
    }

    #[test]
    fn profile_copy() {
        let profile = PhysicsStepProfile {
            total_duration: Duration::from_millis(10),
            active_body_count: 100,
            ..Default::default()
        };

        let copied = profile; // Copy, not move
        assert_eq!(copied.total_duration, profile.total_duration);
    }

    // ============================================================================
    // ASYNC PHYSICS SCHEDULER TESTS (Feature-gated)
    // ============================================================================

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_default_creation() {
        let scheduler = AsyncPhysicsScheduler::new();
        assert_eq!(scheduler.thread_count, 0); // Auto-detect
        assert!(scheduler.enable_profiling);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_with_threads() {
        let scheduler = AsyncPhysicsScheduler::with_threads(4);
        assert_eq!(scheduler.thread_count, 4);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_with_zero_threads() {
        let scheduler = AsyncPhysicsScheduler::with_threads(0);
        assert_eq!(scheduler.thread_count, 0);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_with_many_threads() {
        // Stress test: Unrealistic thread count
        let scheduler = AsyncPhysicsScheduler::with_threads(1000);
        assert_eq!(scheduler.thread_count, 1000);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_get_last_profile_initial() {
        let scheduler = AsyncPhysicsScheduler::new();
        let profile = scheduler.get_last_profile();

        // Should be default (no steps yet)
        assert_eq!(profile.total_duration, Duration::ZERO);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_record_step_telemetry() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        scheduler.record_step_telemetry(Duration::from_millis(16));

        let profile = scheduler.get_last_profile();
        assert_eq!(profile.total_duration, Duration::from_millis(16));
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_record_telemetry_disabled() {
        let mut scheduler = AsyncPhysicsScheduler::new();
        scheduler.enable_profiling = false;

        scheduler.record_step_telemetry(Duration::from_millis(16));

        // Should not record when disabled
        let profile = scheduler.get_last_profile();
        assert_eq!(profile.total_duration, Duration::ZERO);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_step_parallel_simple() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        let profile = scheduler.step_parallel(|| {
            std::thread::sleep(Duration::from_millis(10));
            PhysicsStepProfile {
                active_body_count: 42,
                collision_pair_count: 21,
                solver_iterations: 4,
                ..Default::default()
            }
        });

        assert!(profile.total_duration >= Duration::from_millis(10));
        assert_eq!(profile.active_body_count, 42);
        assert_eq!(profile.collision_pair_count, 21);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_step_parallel_profiling_recorded() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        scheduler.step_parallel(|| PhysicsStepProfile {
            active_body_count: 100,
            ..Default::default()
        });

        let recorded = scheduler.get_last_profile();
        assert_eq!(recorded.active_body_count, 100);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_step_parallel_profiling_disabled() {
        let mut scheduler = AsyncPhysicsScheduler::new();
        scheduler.enable_profiling = false;

        scheduler.step_parallel(|| PhysicsStepProfile {
            active_body_count: 100,
            ..Default::default()
        });

        let recorded = scheduler.get_last_profile();
        // Should not update when profiling disabled
        assert_eq!(recorded.active_body_count, 0);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn scheduler_multiple_steps() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        for i in 1..=5 {
            scheduler.step_parallel(|| PhysicsStepProfile {
                active_body_count: i * 10,
                ..Default::default()
            });

            let profile = scheduler.get_last_profile();
            assert_eq!(profile.active_body_count, i * 10);
        }
    }

    // ============================================================================
    // PARALLEL HELPERS TESTS (Feature-gated)
    // ============================================================================

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_body_processing_deterministic() {
        use super::parallel::par_process_bodies;

        let bodies: Vec<i32> = (0..100).collect();
        let processed = par_process_bodies(&bodies, |&x| x * 2);

        // Results should be in same order as input (determinism)
        for (i, &val) in processed.iter().enumerate() {
            assert_eq!(val, (i as i32) * 2);
        }
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_body_processing_empty() {
        use super::parallel::par_process_bodies;

        let bodies: Vec<i32> = vec![];
        let processed = par_process_bodies(&bodies, |&x| x * 2);

        assert!(processed.is_empty());
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_body_processing_single() {
        use super::parallel::par_process_bodies;

        let bodies: Vec<i32> = vec![42];
        let processed = par_process_bodies(&bodies, |&x| x * 2);

        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0], 84);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_body_processing_large() {
        use super::parallel::par_process_bodies;

        let bodies: Vec<i32> = (0..10000).collect();
        let processed = par_process_bodies(&bodies, |&x| x + 1);

        assert_eq!(processed.len(), 10000);

        // Verify all processed correctly
        for (i, &val) in processed.iter().enumerate() {
            assert_eq!(val, (i as i32) + 1);
        }
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_collision_pairs_execution() {
        use super::parallel::par_process_collision_pairs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pairs: Vec<i32> = (0..100).collect();
        let counter = AtomicUsize::new(0);

        par_process_collision_pairs(&pairs, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 100);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn parallel_collision_pairs_empty() {
        use super::parallel::par_process_collision_pairs;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let pairs: Vec<i32> = vec![];
        let counter = AtomicUsize::new(0);

        par_process_collision_pairs(&pairs, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    // ============================================================================
    // 3-STAGE PARALLEL PIPELINE TESTS (New)
    // ============================================================================

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_basic_flow() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        let profile = scheduler.step_parallel_staged(
            || {
                // Broad-phase: produce 10 candidate pairs
                vec![0u32; 10]
            },
            |pairs| {
                // Narrow-phase: filter to 5 contacts
                assert_eq!(pairs.len(), 10);
                pairs[..5].to_vec()
            },
            |contacts| {
                // Integration: consume contacts
                assert_eq!(contacts.len(), 5);
            },
        );

        assert!(profile.total_duration > Duration::ZERO);
        assert!(
            profile.broad_phase_duration > Duration::ZERO
                || profile.narrow_phase_duration > Duration::ZERO
        );
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_per_stage_timing() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        let profile = scheduler.step_parallel_staged(
            || {
                std::thread::sleep(Duration::from_millis(5));
                42u32
            },
            |value| {
                std::thread::sleep(Duration::from_millis(10));
                assert_eq!(value, 42);
                "contacts"
            },
            |msg| {
                std::thread::sleep(Duration::from_millis(5));
                assert_eq!(msg, "contacts");
            },
        );

        // Narrow-phase (10ms) should dominate
        assert!(
            profile.narrow_phase_duration >= Duration::from_millis(9),
            "Narrow-phase should be ~10ms, got {:?}",
            profile.narrow_phase_duration
        );
        // Total should be >= sum of stages
        assert!(
            profile.total_duration >= Duration::from_millis(19),
            "Total should be >=20ms, got {:?}",
            profile.total_duration
        );
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_profiling_recorded() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        scheduler.step_parallel_staged(|| vec![1, 2, 3], |v| v.len(), |_| {});

        let recorded = scheduler.get_last_profile();
        assert!(recorded.total_duration > Duration::ZERO);
        // Per-stage durations should be set
        // (they may be very small but at least broad_phase + narrow_phase + integration >= 0)
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_profiling_disabled() {
        let mut scheduler = AsyncPhysicsScheduler::new();
        scheduler.enable_profiling = false;

        scheduler.step_parallel_staged(|| 42, |x| x * 2, |_| {});

        let recorded = scheduler.get_last_profile();
        assert_eq!(
            recorded.total_duration,
            Duration::ZERO,
            "Profiling disabled should not record"
        );
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_data_flows_through_stages() {
        let mut scheduler = AsyncPhysicsScheduler::new();
        use std::sync::atomic::{AtomicUsize, Ordering};

        let integration_received = std::sync::Arc::new(AtomicUsize::new(0));
        let integration_received_clone = integration_received.clone();

        scheduler.step_parallel_staged(
            || {
                // Broad-phase produces 100 pairs
                (0..100).collect::<Vec<u32>>()
            },
            |pairs| {
                // Narrow-phase filters even pairs
                pairs
                    .into_iter()
                    .filter(|p| p % 2 == 0)
                    .collect::<Vec<u32>>()
            },
            move |contacts| {
                // Integration receives 50 even contacts
                integration_received_clone.store(contacts.len(), Ordering::SeqCst);
            },
        );

        assert_eq!(integration_received.load(Ordering::SeqCst), 50);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_with_rayon_broad_phase() {
        use super::parallel::par_process_bodies;

        let mut scheduler = AsyncPhysicsScheduler::new();

        let profile = scheduler.step_parallel_staged(
            || {
                // Broad-phase: parallel AABB expansion
                let bodies: Vec<f32> = (0..1000).map(|i| i as f32).collect();
                par_process_bodies(&bodies, |&x| x * 2.0)
            },
            |expanded| {
                // Narrow-phase: count overlaps
                assert_eq!(expanded.len(), 1000);
                expanded.len()
            },
            |count| {
                assert_eq!(count, 1000);
            },
        );

        assert!(profile.total_duration > Duration::ZERO);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_empty_stages() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        // All stages are no-ops
        let profile = scheduler.step_parallel_staged(|| (), |_| (), |_| {});

        assert!(profile.broad_phase_duration <= Duration::from_millis(1));
        assert!(profile.narrow_phase_duration <= Duration::from_millis(1));
        assert!(profile.integration_duration <= Duration::from_millis(1));
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_multiple_iterations() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        for frame in 0..10 {
            let profile = scheduler.step_parallel_staged(|| frame, |f| f * 2, |_| {});

            assert!(profile.total_duration > Duration::ZERO);
            let recorded = scheduler.get_last_profile();
            assert!(recorded.total_duration > Duration::ZERO);
        }
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_pipeline_stage_ordering_enforced() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let mut scheduler = AsyncPhysicsScheduler::new();
        let sequence = std::sync::Arc::new(AtomicU32::new(0));

        let s1 = sequence.clone();
        let s2 = sequence.clone();
        let s3 = sequence.clone();

        scheduler.step_parallel_staged(
            move || {
                // Broad-phase runs first (sequence = 0 → 1)
                assert_eq!(s1.fetch_add(1, Ordering::SeqCst), 0);
            },
            move |_| {
                // Narrow-phase runs second (sequence = 1 → 2)
                assert_eq!(s2.fetch_add(1, Ordering::SeqCst), 1);
            },
            move |_| {
                // Integration runs third (sequence = 2 → 3)
                assert_eq!(s3.fetch_add(1, Ordering::SeqCst), 2);
            },
        );

        assert_eq!(sequence.load(Ordering::SeqCst), 3);
    }

    // ============================================================================
    // STAGED PIPELINE WITH STATS TESTS
    // ============================================================================

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_with_stats_records_counts() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        let profile = scheduler.step_parallel_staged_with_stats(
            || vec![1, 2, 3],
            |v| v.len(),
            |_| {},
            500, // active bodies
            120, // collision pairs
            8,   // solver iterations
        );

        assert_eq!(profile.active_body_count, 500);
        assert_eq!(profile.collision_pair_count, 120);
        assert_eq!(profile.solver_iterations, 8);
        assert!(profile.total_duration > Duration::ZERO);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_with_stats_updates_last_profile() {
        let mut scheduler = AsyncPhysicsScheduler::new();

        scheduler.step_parallel_staged_with_stats(|| (), |_| (), |_| {}, 200, 50, 4);

        let recorded = scheduler.get_last_profile();
        assert_eq!(recorded.active_body_count, 200);
        assert_eq!(recorded.collision_pair_count, 50);
        assert_eq!(recorded.solver_iterations, 4);
    }

    #[test]
    #[cfg(feature = "async-physics")]
    fn staged_with_stats_profiling_disabled() {
        let mut scheduler = AsyncPhysicsScheduler::new();
        scheduler.enable_profiling = false;

        scheduler.step_parallel_staged_with_stats(|| (), |_| (), |_| {}, 200, 50, 4);

        let recorded = scheduler.get_last_profile();
        // Stats should NOT be recorded when profiling disabled
        assert_eq!(recorded.active_body_count, 0);
    }
}
