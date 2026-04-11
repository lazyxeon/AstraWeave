//! Rain impact splash particle system.
//!
//! Runs a compute pass after rain occlusion to detect rain particles that
//! impacted surfaces, then spawns small radial splash bursts at each impact
//! position.
//!
//! The splash particles are stored in a separate buffer and rendered
//! as translucent point sprites via the existing GPU particle renderer.

use bytemuck::{Pod, Zeroable};

// ── Constants ───────────────────────────────────────────────────────────────

/// WGSL shader source.
const RAIN_SPLASH_WGSL: &str = include_str!("../shaders/rain_splash.wgsl");

/// Default maximum splash particles (short-lived, so doesn't need to be huge).
const DEFAULT_MAX_SPLASH: u32 = 8192;

// ── GPU structs ─────────────────────────────────────────────────────────────

/// Splash spawn parameters (32 bytes, matches WGSL `SplashParams`).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct SplashParams {
    pub rain_particle_count: u32,
    pub max_splash_particles: u32,
    pub splash_per_impact: u32,
    pub splash_lifetime: f32,
    pub splash_speed: f32,
    pub splash_scale: f32,
    pub dt: f32,
    pub random_seed: u32,
}

impl Default for SplashParams {
    fn default() -> Self {
        Self {
            rain_particle_count: 0,
            max_splash_particles: DEFAULT_MAX_SPLASH,
            splash_per_impact: 5,
            splash_lifetime: 0.3,
            splash_speed: 1.5,
            splash_scale: 0.02,
            dt: 1.0 / 60.0,
            random_seed: 0,
        }
    }
}

// ── Configuration ───────────────────────────────────────────────────────────

/// CPU-side splash configuration.
#[derive(Debug, Clone, Copy)]
pub struct RainSplashConfig {
    /// Number of splash particles per raindrop impact.
    pub splash_per_impact: u32,
    /// Splash particle lifetime in seconds.
    pub splash_lifetime: f32,
    /// Outward radial speed of splash particles.
    pub splash_speed: f32,
    /// Visual scale of each splash particle.
    pub splash_scale: f32,
    /// Maximum splash particles in the buffer.
    pub max_splash_particles: u32,
}

impl Default for RainSplashConfig {
    fn default() -> Self {
        Self {
            splash_per_impact: 5,
            splash_lifetime: 0.3,
            splash_speed: 1.5,
            splash_scale: 0.02,
            max_splash_particles: DEFAULT_MAX_SPLASH,
        }
    }
}

impl RainSplashConfig {
    /// Build GPU params from config + runtime state.
    pub fn to_gpu_params(
        &self,
        rain_particle_count: u32,
        dt: f32,
        frame_seed: u32,
    ) -> SplashParams {
        SplashParams {
            rain_particle_count,
            max_splash_particles: self.max_splash_particles,
            splash_per_impact: self.splash_per_impact,
            splash_lifetime: self.splash_lifetime,
            splash_speed: self.splash_speed,
            splash_scale: self.splash_scale,
            dt,
            random_seed: frame_seed,
        }
    }

    /// Heavy rain preset — more splashes, bigger.
    pub fn heavy() -> Self {
        Self {
            splash_per_impact: 8,
            splash_lifetime: 0.4,
            splash_speed: 2.0,
            splash_scale: 0.03,
            max_splash_particles: 16384,
        }
    }

    /// Light drizzle preset — fewer, smaller splashes.
    pub fn drizzle() -> Self {
        Self {
            splash_per_impact: 3,
            splash_lifetime: 0.2,
            splash_speed: 1.0,
            splash_scale: 0.015,
            max_splash_particles: 4096,
        }
    }

    /// Compute dispatch workgroup count for the spawn shader.
    pub fn dispatch_size(&self, rain_particle_count: u32) -> u32 {
        rain_particle_count.div_ceil(64)
    }
}

// ── WGSL Source ─────────────────────────────────────────────────────────────

/// Compile-time splash shader source.
pub const RAIN_SPLASH_SHADER: &str = RAIN_SPLASH_WGSL;

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_params_size() {
        assert_eq!(std::mem::size_of::<SplashParams>(), 32);
    }

    #[test]
    fn splash_params_default() {
        let p = SplashParams::default();
        assert_eq!(p.splash_per_impact, 5);
        assert!(p.splash_lifetime > 0.0);
        assert!(p.splash_speed > 0.0);
    }

    #[test]
    fn splash_params_bytemuck_roundtrip() {
        let p = SplashParams {
            rain_particle_count: 5000,
            max_splash_particles: 8192,
            splash_per_impact: 6,
            splash_lifetime: 0.35,
            splash_speed: 1.8,
            splash_scale: 0.025,
            dt: 0.016,
            random_seed: 42,
        };
        let bytes = bytemuck::bytes_of(&p);
        assert_eq!(bytes.len(), 32);
        let back: &SplashParams = bytemuck::from_bytes(bytes);
        assert_eq!(back.rain_particle_count, 5000);
        assert_eq!(back.random_seed, 42);
    }

    #[test]
    fn splash_config_to_gpu_params() {
        let cfg = RainSplashConfig::default();
        let params = cfg.to_gpu_params(1000, 0.016, 42);
        assert_eq!(params.rain_particle_count, 1000);
        assert_eq!(params.random_seed, 42);
        assert!((params.dt - 0.016).abs() < f32::EPSILON);
    }

    #[test]
    fn splash_config_presets() {
        let heavy = RainSplashConfig::heavy();
        let drizzle = RainSplashConfig::drizzle();
        assert!(heavy.splash_per_impact > drizzle.splash_per_impact);
        assert!(heavy.max_splash_particles > drizzle.max_splash_particles);
    }

    #[test]
    fn splash_dispatch_size() {
        let cfg = RainSplashConfig::default();
        assert_eq!(cfg.dispatch_size(0), 0);
        assert_eq!(cfg.dispatch_size(64), 1);
        assert_eq!(cfg.dispatch_size(65), 2);
    }

    #[test]
    fn splash_wgsl_present() {
        assert!(RAIN_SPLASH_WGSL.contains("spawn_splashes"));
        assert!(RAIN_SPLASH_WGSL.contains("SplashParams"));
        assert!(RAIN_SPLASH_WGSL.contains("splash_particles"));
    }
}
