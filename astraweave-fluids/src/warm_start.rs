// =============================================================================
// Warm-Starting System for Pressure Solvers
// =============================================================================
//
// Warm-starting accelerates iterative pressure solvers by using the previous
// frame's solution as an initial guess. This is based on temporal coherence:
// pressure fields typically don't change dramatically between frames.
//
// References:
// - Macklin & Müller 2013: "Position Based Fluids"
// - Bender & Koschier 2017: "Divergence-Free SPH"
// - Ihmsen et al. 2014: "Implicit Incompressible SPH"
//
// Benefits:
// - 30-50% reduction in solver iterations
// - More stable convergence
// - Better temporal coherence in pressure fields
//
// This module provides:
// - Pressure history tracking with configurable decay
// - Adaptive relaxation based on scene dynamics
// - GPU-friendly buffer management
// - Statistics for monitoring warm-start effectiveness
//
// =============================================================================

use crate::research::{ResearchParticle, ResearchSimParams, SolverType};

/// Configuration for warm-starting behavior
#[derive(Clone, Debug)]
pub struct WarmStartConfig {
    /// Enable warm-starting
    pub enabled: bool,

    /// Relaxation factor for initial pressure guess (0.0 = ignore history, 1.0 = full history)
    pub relaxation: f32,

    /// Decay factor per frame (for when particles move significantly)
    pub temporal_decay: f32,

    /// Maximum pressure value to clamp (prevents instability from old data)
    pub max_pressure_clamp: f32,

    /// Minimum frames between full resets
    pub min_frames_between_reset: u32,

    /// Enable adaptive relaxation based on velocity changes
    pub adaptive_relaxation: bool,

    /// Velocity threshold for reducing relaxation (m/s)
    pub velocity_threshold: f32,

    /// Minimum relaxation when velocity is high
    pub min_relaxation: f32,
}

impl Default for WarmStartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            relaxation: 0.8,
            temporal_decay: 0.95,
            max_pressure_clamp: 1e6,
            min_frames_between_reset: 10,
            adaptive_relaxation: true,
            velocity_threshold: 5.0,
            min_relaxation: 0.3,
        }
    }
}

impl WarmStartConfig {
    /// Create configuration for conservative warm-starting (more stable)
    #[must_use]
    pub fn conservative() -> Self {
        Self {
            enabled: true,
            relaxation: 0.5,
            temporal_decay: 0.9,
            max_pressure_clamp: 1e5,
            adaptive_relaxation: true,
            min_relaxation: 0.2,
            ..Default::default()
        }
    }

    /// Create configuration for aggressive warm-starting (faster convergence)
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            relaxation: 0.95,
            temporal_decay: 0.98,
            max_pressure_clamp: 1e7,
            adaptive_relaxation: false,
            ..Default::default()
        }
    }

    /// Create configuration optimized for PCISPH solver
    #[must_use]
    pub fn for_pcisph() -> Self {
        Self {
            enabled: true,
            relaxation: 0.7,
            temporal_decay: 0.92,
            max_pressure_clamp: 5e5,
            adaptive_relaxation: true,
            velocity_threshold: 3.0,
            min_relaxation: 0.4,
            ..Default::default()
        }
    }

    /// Create configuration optimized for DFSPH solver
    #[must_use]
    pub fn for_dfsph() -> Self {
        Self {
            enabled: true,
            relaxation: 0.85,
            temporal_decay: 0.95,
            max_pressure_clamp: 1e6,
            adaptive_relaxation: true,
            velocity_threshold: 4.0,
            min_relaxation: 0.35,
            ..Default::default()
        }
    }

    /// Disable warm-starting entirely
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }
}

/// Statistics tracking for warm-start effectiveness
#[derive(Clone, Debug, Default)]
pub struct WarmStartStats {
    /// Number of frames processed
    pub frames_processed: u32,

    /// Average iteration reduction (percentage)
    pub avg_iteration_reduction: f32,

    /// Number of times warm-start was reset
    pub reset_count: u32,

    /// Current effective relaxation factor
    pub current_relaxation: f32,

    /// Average max velocity in recent frames
    pub avg_max_velocity: f32,

    /// Baseline iterations (without warm-start, estimated)
    pub baseline_iterations: u32,

    /// Actual iterations (with warm-start)
    pub actual_iterations: u32,
}

impl WarmStartStats {
    /// Compute the iteration reduction percentage
    #[must_use]
    pub fn iteration_reduction_percent(&self) -> f32 {
        if self.baseline_iterations == 0 {
            return 0.0;
        }
        let reduction = self
            .baseline_iterations
            .saturating_sub(self.actual_iterations);
        100.0 * reduction as f32 / self.baseline_iterations as f32
    }
}

/// GPU-aligned pressure history buffer entry
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PressureHistoryEntry {
    /// Previous pressure value
    pub previous_pressure: f32,

    /// Previous density error (for PCISPH)
    pub previous_density_error: f32,

    /// Previous velocity divergence (for DFSPH)
    pub previous_divergence: f32,

    /// Frame age (how many frames since last update)
    pub age: f32,
}

/// Warm-starting system for pressure solvers
pub struct WarmStartSystem {
    config: WarmStartConfig,
    stats: WarmStartStats,
    frames_since_reset: u32,
    velocity_history: Vec<f32>, // Ring buffer of max velocities
    velocity_idx: usize,
}

impl WarmStartSystem {
    /// Create a new warm-start system
    #[must_use]
    pub fn new(config: WarmStartConfig) -> Self {
        Self {
            config,
            stats: WarmStartStats::default(),
            frames_since_reset: 0,
            velocity_history: vec![0.0; 8], // 8-frame history
            velocity_idx: 0,
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn default_config() -> Self {
        Self::new(WarmStartConfig::default())
    }

    /// Create with solver-specific configuration
    #[must_use]
    pub fn for_solver(solver: SolverType) -> Self {
        let config = match solver {
            SolverType::PCISPH => WarmStartConfig::for_pcisph(),
            // (`for_dfsph` remains available for direct use; the DFSPH
            // SolverType variant was removed in F.1 — never implemented.)
            _ => WarmStartConfig::default(),
        };
        Self::new(config)
    }

    /// Initialize particles with warm-start data from previous frame
    ///
    /// This should be called at the start of each solver iteration.
    pub fn apply_warm_start(
        &mut self,
        particles: &mut [ResearchParticle],
        _params: &ResearchSimParams,
    ) {
        if !self.config.enabled {
            return;
        }

        // Compute adaptive relaxation based on max velocity
        let max_velocity = self.compute_max_velocity(particles);
        self.update_velocity_history(max_velocity);

        let effective_relaxation = if self.config.adaptive_relaxation {
            self.compute_adaptive_relaxation(max_velocity)
        } else {
            self.config.relaxation
        };

        self.stats.current_relaxation = effective_relaxation;

        // Apply warm-start to each particle
        for p in particles.iter_mut() {
            // Use previous_pressure field already in ResearchParticle
            let warm_pressure = p.previous_pressure * effective_relaxation;

            // Clamp to prevent instability
            let clamped = warm_pressure.clamp(
                -self.config.max_pressure_clamp,
                self.config.max_pressure_clamp,
            );

            // Apply decay
            p.previous_pressure = clamped * self.config.temporal_decay;
        }

        self.frames_since_reset += 1;
        self.stats.frames_processed += 1;
    }

    /// Store current pressure values for next frame
    ///
    /// Call this at the end of the solver after convergence.
    pub fn store_pressure_history(&mut self, particles: &mut [ResearchParticle]) {
        if !self.config.enabled {
            return;
        }

        // The pressure is already stored in particle.previous_pressure during solver
        // This method is for any additional post-processing

        for p in particles.iter_mut() {
            // Apply decay to stored pressure
            p.previous_pressure *= self.config.temporal_decay;

            // Clamp to valid range
            p.previous_pressure = p.previous_pressure.clamp(
                -self.config.max_pressure_clamp,
                self.config.max_pressure_clamp,
            );
        }
    }

    /// Reset warm-start history (call when scene changes dramatically)
    pub fn reset(&mut self, particles: &mut [ResearchParticle]) {
        for p in particles.iter_mut() {
            p.previous_pressure = 0.0;
        }

        self.frames_since_reset = 0;
        self.stats.reset_count += 1;
        self.velocity_history.fill(0.0);
    }

    /// Check if a reset is recommended based on current conditions
    #[must_use]
    pub fn should_reset(&self, max_velocity: f32) -> bool {
        // Don't reset too frequently
        if self.frames_since_reset < self.config.min_frames_between_reset {
            return false;
        }

        // Reset if velocity is extremely high (explosive scenario)
        let velocity_threshold = self.config.velocity_threshold * 5.0;
        max_velocity > velocity_threshold
    }

    /// Record iteration count for statistics
    pub fn record_iterations(&mut self, baseline: u32, actual: u32) {
        self.stats.baseline_iterations = baseline;
        self.stats.actual_iterations = actual;

        // Update running average of iteration reduction
        let reduction = self.stats.iteration_reduction_percent();
        let alpha = 0.1; // Smoothing factor
        self.stats.avg_iteration_reduction =
            self.stats.avg_iteration_reduction * (1.0 - alpha) + reduction * alpha;
    }

    /// Get current statistics
    #[must_use]
    pub fn stats(&self) -> &WarmStartStats {
        &self.stats
    }

    /// Get current configuration
    #[must_use]
    pub fn config(&self) -> &WarmStartConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: WarmStartConfig) {
        self.config = config;
    }

    /// Check if warm-starting is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable warm-starting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    fn compute_max_velocity(&self, particles: &[ResearchParticle]) -> f32 {
        particles
            .iter()
            .map(|p| {
                let vx = p.velocity[0];
                let vy = p.velocity[1];
                let vz = p.velocity[2];
                (vx * vx + vy * vy + vz * vz).sqrt()
            })
            .fold(0.0f32, f32::max)
    }

    fn update_velocity_history(&mut self, max_velocity: f32) {
        self.velocity_history[self.velocity_idx] = max_velocity;
        self.velocity_idx = (self.velocity_idx + 1) % self.velocity_history.len();

        // Update average
        let sum: f32 = self.velocity_history.iter().sum();
        self.stats.avg_max_velocity = sum / self.velocity_history.len() as f32;
    }

    fn compute_adaptive_relaxation(&self, max_velocity: f32) -> f32 {
        if max_velocity < self.config.velocity_threshold {
            // Low velocity: use full relaxation
            self.config.relaxation
        } else {
            // High velocity: reduce relaxation linearly
            let excess =
                (max_velocity - self.config.velocity_threshold) / self.config.velocity_threshold;
            let factor = 1.0 - excess.min(1.0);

            // Interpolate between min_relaxation and relaxation
            self.config.min_relaxation
                + factor * (self.config.relaxation - self.config.min_relaxation)
        }
    }
}

/// Warm-start integration helper for PCISPH solver
pub struct PcisphWarmStart;

impl PcisphWarmStart {
    /// Initialize pressure prediction for PCISPH
    ///
    /// Uses previous frame's pressure as initial guess scaled by relaxation.
    pub fn initialize_pressure(
        particles: &mut [ResearchParticle],
        relaxation: f32,
        max_clamp: f32,
    ) {
        for p in particles.iter_mut() {
            // Scale previous pressure by relaxation factor
            let warm_pressure = p.previous_pressure * relaxation;

            // Clamp to valid range
            p.previous_pressure = warm_pressure.clamp(-max_clamp, max_clamp);
        }
    }

    /// Store converged pressure for next frame
    pub fn store_converged_pressure(
        particles: &mut [ResearchParticle],
        current_pressures: &[f32],
        decay: f32,
    ) {
        for (p, &pressure) in particles.iter_mut().zip(current_pressures.iter()) {
            p.previous_pressure = pressure * decay;
        }
    }
}

/// Warm-start integration helper for DFSPH solver
pub struct DfsphWarmStart;

impl DfsphWarmStart {
    /// Initialize α and κ factors for DFSPH
    ///
    /// DFSPH uses these factors for density and divergence correction.
    pub fn initialize_factors(particles: &mut [ResearchParticle], relaxation: f32) {
        for p in particles.iter_mut() {
            // Scale factors by relaxation
            p.alpha *= relaxation;
            p.kappa *= relaxation;
        }
    }

    /// Store converged factors for next frame
    pub fn store_converged_factors(
        particles: &mut [ResearchParticle],
        alphas: &[f32],
        kappas: &[f32],
        decay: f32,
    ) {
        for (i, p) in particles.iter_mut().enumerate() {
            if i < alphas.len() {
                p.alpha = alphas[i] * decay;
            }
            if i < kappas.len() {
                p.kappa = kappas[i] * decay;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warm_start_config_defaults() {
        let config = WarmStartConfig::default();
        assert!(config.enabled);
        assert!((config.relaxation - 0.8).abs() < f32::EPSILON);
        assert!((config.temporal_decay - 0.95).abs() < f32::EPSILON);
        assert!(config.adaptive_relaxation);
    }

    #[test]
    fn test_warm_start_config_presets() {
        let conservative = WarmStartConfig::conservative();
        assert!(conservative.relaxation < WarmStartConfig::default().relaxation);

        let aggressive = WarmStartConfig::aggressive();
        assert!(aggressive.relaxation > WarmStartConfig::default().relaxation);

        let pcisph = WarmStartConfig::for_pcisph();
        assert!(pcisph.enabled);

        let dfsph = WarmStartConfig::for_dfsph();
        assert!(dfsph.enabled);

        let disabled = WarmStartConfig::disabled();
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_warm_start_system_creation() {
        let system = WarmStartSystem::default_config();
        assert!(system.is_enabled());
        assert_eq!(system.stats().frames_processed, 0);
    }

    #[test]
    fn test_warm_start_for_solver() {
        let pcisph = WarmStartSystem::for_solver(SolverType::PCISPH);
        assert!((pcisph.config().relaxation - 0.7).abs() < f32::EPSILON);

        // DFSPH SolverType variant removed in F.1; the config preset remains
        // directly constructible.
        let dfsph = WarmStartSystem::new(WarmStartConfig::for_dfsph());
        assert!((dfsph.config().relaxation - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_pressure_history_entry_size() {
        assert_eq!(std::mem::size_of::<PressureHistoryEntry>(), 16);
    }

    #[test]
    fn test_apply_warm_start_disabled() {
        let config = WarmStartConfig::disabled();
        let mut system = WarmStartSystem::new(config);

        let mut particles = vec![ResearchParticle::default(); 10];
        for (i, p) in particles.iter_mut().enumerate() {
            p.previous_pressure = i as f32 * 100.0;
        }
        let params = ResearchSimParams::default();

        // Store original pressures
        let original: Vec<f32> = particles.iter().map(|p| p.previous_pressure).collect();

        system.apply_warm_start(&mut particles, &params);

        // Pressures should be unchanged when disabled
        for (p, orig) in particles.iter().zip(original.iter()) {
            assert!((p.previous_pressure - orig).abs() < 1e-6);
        }
    }

    #[test]
    fn test_apply_warm_start_enabled() {
        let mut system = WarmStartSystem::default_config();

        let mut particles = vec![];
        for i in 0..10 {
            let mut p = ResearchParticle::default();
            p.previous_pressure = i as f32 * 100.0;
            p.velocity = [0.0, 0.0, 0.0, 0.0]; // Low velocity
            particles.push(p);
        }
        let params = ResearchSimParams::default();

        system.apply_warm_start(&mut particles, &params);

        // Pressures should be scaled and decayed
        assert!(system.stats().frames_processed == 1);
        assert!(system.stats().current_relaxation > 0.0);
    }

    #[test]
    fn test_warm_start_reset() {
        let mut system = WarmStartSystem::default_config();

        let mut particles = vec![];
        for i in 0..5 {
            let mut p = ResearchParticle::default();
            p.previous_pressure = (i as f32 + 1.0) * 100.0;
            particles.push(p);
        }

        system.reset(&mut particles);

        // All pressures should be zero
        for p in &particles {
            assert!(p.previous_pressure.abs() < 1e-10);
        }
        assert_eq!(system.stats().reset_count, 1);
    }

    #[test]
    fn test_warm_start_clamp() {
        let mut config = WarmStartConfig::default();
        config.max_pressure_clamp = 100.0;
        let mut system = WarmStartSystem::new(config);

        let mut particles = vec![];
        let mut p = ResearchParticle::default();
        p.previous_pressure = 1000.0; // Much higher than clamp
        p.velocity = [0.0, 0.0, 0.0, 0.0];
        particles.push(p);

        let params = ResearchSimParams::default();
        system.apply_warm_start(&mut particles, &params);

        // Should be clamped (after scaling and decay)
        assert!(particles[0].previous_pressure <= 100.0);
    }

    #[test]
    fn test_adaptive_relaxation() {
        let config = WarmStartConfig {
            adaptive_relaxation: true,
            relaxation: 0.8,
            min_relaxation: 0.2,
            velocity_threshold: 5.0,
            ..Default::default()
        };
        let mut system = WarmStartSystem::new(config);

        // Low velocity: should use full relaxation
        let mut particles_low = vec![ResearchParticle::default()];
        particles_low[0].velocity = [1.0, 0.0, 0.0, 0.0]; // 1 m/s
        let params = ResearchSimParams::default();

        system.apply_warm_start(&mut particles_low, &params);
        assert!((system.stats().current_relaxation - 0.8).abs() < 0.01);

        // High velocity: should reduce relaxation
        let mut particles_high = vec![ResearchParticle::default()];
        particles_high[0].velocity = [15.0, 0.0, 0.0, 0.0]; // 15 m/s

        system.apply_warm_start(&mut particles_high, &params);
        assert!(system.stats().current_relaxation < 0.8);
    }

    #[test]
    fn test_record_iterations() {
        let mut system = WarmStartSystem::default_config();

        system.record_iterations(10, 7);

        assert_eq!(system.stats().baseline_iterations, 10);
        assert_eq!(system.stats().actual_iterations, 7);
        assert!((system.stats().iteration_reduction_percent() - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_iteration_reduction_zero_baseline() {
        let stats = WarmStartStats::default();
        assert!(stats.iteration_reduction_percent().abs() < f32::EPSILON);
    }

    #[test]
    fn test_should_reset() {
        let config = WarmStartConfig {
            min_frames_between_reset: 5,
            velocity_threshold: 5.0,
            ..Default::default()
        };
        let mut system = WarmStartSystem::new(config);

        // Before min frames
        assert!(!system.should_reset(100.0));

        // Simulate frames
        for _ in 0..10 {
            system.frames_since_reset += 1;
        }

        // After min frames with high velocity
        assert!(system.should_reset(30.0)); // 5 * 5.0 = 25, 30 > 25

        // After min frames with low velocity
        assert!(!system.should_reset(10.0));
    }

    #[test]
    fn test_pcisph_warm_start_initialize() {
        let mut particles = vec![];
        for i in 0..5 {
            let mut p = ResearchParticle::default();
            p.previous_pressure = (i as f32 + 1.0) * 100.0;
            particles.push(p);
        }

        PcisphWarmStart::initialize_pressure(&mut particles, 0.5, 1000.0);

        // Pressures should be halved (0.5 relaxation)
        assert!((particles[0].previous_pressure - 50.0).abs() < 1.0);
        assert!((particles[4].previous_pressure - 250.0).abs() < 1.0);
    }

    #[test]
    fn test_dfsph_warm_start_initialize() {
        let mut particles = vec![];
        for i in 0..5 {
            let mut p = ResearchParticle::default();
            p.alpha = (i as f32 + 1.0) * 0.1;
            p.kappa = (i as f32 + 1.0) * 0.05;
            particles.push(p);
        }

        DfsphWarmStart::initialize_factors(&mut particles, 0.5);

        // Factors should be halved
        assert!((particles[0].alpha - 0.05).abs() < 0.01);
        assert!((particles[0].kappa - 0.025).abs() < 0.01);
    }

    #[test]
    fn test_store_pressure_history() {
        let mut system = WarmStartSystem::default_config();

        let mut particles = vec![];
        for i in 0..5 {
            let mut p = ResearchParticle::default();
            p.previous_pressure = (i as f32 + 1.0) * 100.0;
            particles.push(p);
        }

        system.store_pressure_history(&mut particles);

        // Pressures should be decayed
        // decay = 0.95, so pressure[0] = 100 * 0.95 = 95
        assert!((particles[0].previous_pressure - 95.0).abs() < 1.0);
    }

    #[test]
    fn test_velocity_history_update() {
        let mut system = WarmStartSystem::default_config();

        // Create particles with known velocity
        let mut particles = vec![ResearchParticle::default()];
        particles[0].velocity = [3.0, 4.0, 0.0, 0.0]; // magnitude = 5.0

        let params = ResearchSimParams::default();
        system.apply_warm_start(&mut particles, &params);

        // Check that velocity history was updated
        // avg_max_velocity should be 5.0/8 = 0.625 (first entry in 8-slot buffer)
        assert!(system.stats().avg_max_velocity > 0.0);
    }
}
