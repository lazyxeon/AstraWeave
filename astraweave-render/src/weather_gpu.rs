//! GPU-accelerated weather particle emitter configurations.
//!
//! Bridges the high-level `WeatherSystem` (environment.rs) with the GPU
//! compute particle simulation (`GpuParticleSystem` + `simulate.wgsl`).
//!
//! Each `WeatherType` maps to a `WeatherParticleConfig` that populates
//! `EmitterParams` for the GPU pipeline. The CPU path in `effects.rs`
//! remains available behind `#[cfg(feature = "cpu_particles")]`.

use crate::gpu_particles::EmitterParams;
use glam::Vec3;
use serde::{Deserialize, Serialize};

// ── Weather particle types ──────────────────────────────────────────────────

/// Precipitation type selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherParticleType {
    Rain,
    Snow,
    Sandstorm,
    WindTrails,
}

/// GPU-ready weather particle emitter configuration.
///
/// Translates high-level weather semantics (rain intensity, spawn radius)
/// into the low-level `EmitterParams` consumed by the GPU compute pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherParticleConfig {
    /// Type of precipitation.
    pub particle_type: WeatherParticleType,

    /// Particles spawned per second.
    pub spawn_rate: u32,

    /// Spawn height above the camera (metres).
    pub spawn_height_above_camera: f32,

    /// Horizontal radius around the camera where particles spawn (metres).
    pub spawn_radius: f32,

    /// Base velocity (world-space, Y is typically negative for falling).
    pub velocity: Vec3,

    /// How strongly wind displaces the spawn velocity (0.0–1.0).
    pub wind_influence: f32,

    /// Base particle lifetime in seconds.
    pub lifetime: f32,

    /// Velocity randomness factor (0.0 = uniform, 1.0 = ±100% jitter).
    pub velocity_randomness: f32,

    /// Base colour (linear RGBA).
    pub color: [f32; 4],

    /// Particle scale (xyz).
    pub scale: [f32; 3],

    /// Gravity multiplier applied per particle (negative Y = downward).
    pub gravity: [f32; 3],

    /// Maximum particle count (hard cap for GPU buffer sizing).
    pub max_particles: u32,
}

impl WeatherParticleConfig {
    /// Preset for rain (matches CPU `tick_rain` in effects.rs).
    pub fn rain() -> Self {
        Self {
            particle_type: WeatherParticleType::Rain,
            spawn_rate: 5000,
            spawn_height_above_camera: 15.0,
            spawn_radius: 30.0,
            velocity: Vec3::new(0.0, -20.0, 0.0),
            wind_influence: 0.8,
            lifetime: 1.0,
            velocity_randomness: 0.3,
            color: [0.85, 0.9, 1.0, 0.6],
            scale: [0.015, 0.6, 0.015],
            gravity: [0.0, -9.81, 0.0],
            max_particles: 20_000,
        }
    }

    /// Preset for snow (matches CPU `tick_snow`).
    pub fn snow() -> Self {
        Self {
            particle_type: WeatherParticleType::Snow,
            spawn_rate: 3000,
            spawn_height_above_camera: 18.0,
            spawn_radius: 35.0,
            velocity: Vec3::new(0.0, -2.0, 0.0),
            wind_influence: 0.5,
            lifetime: 4.5,
            velocity_randomness: 0.25,
            color: [1.0, 1.0, 1.0, 0.85],
            scale: [0.08, 0.08, 0.08],
            gravity: [0.0, -1.0, 0.0],
            max_particles: 15_000,
        }
    }

    /// Preset for sandstorm (matches CPU `tick_sandstorm`).
    pub fn sandstorm() -> Self {
        Self {
            particle_type: WeatherParticleType::Sandstorm,
            spawn_rate: 8000,
            spawn_height_above_camera: 5.0,
            spawn_radius: 50.0,
            velocity: Vec3::new(15.0, 0.5, 0.0),
            wind_influence: 1.0,
            lifetime: 1.6,
            velocity_randomness: 0.4,
            color: [0.85, 0.75, 0.55, 0.7],
            scale: [0.03, 0.03, 0.15],
            gravity: [0.0, -2.0, 0.0],
            max_particles: 30_000,
        }
    }

    /// Preset for wind trails (matches CPU `tick_wind`).
    pub fn wind_trails() -> Self {
        Self {
            particle_type: WeatherParticleType::WindTrails,
            spawn_rate: 1000,
            spawn_height_above_camera: 2.0,
            spawn_radius: 30.0,
            velocity: Vec3::new(8.0, 0.0, 0.0),
            wind_influence: 1.0,
            lifetime: 2.0,
            velocity_randomness: 0.3,
            color: [1.0, 1.0, 1.0, 0.3],
            scale: [0.05, 0.05, 0.8],
            gravity: [0.0, 0.0, 0.0],
            max_particles: 5_000,
        }
    }

    /// Select an appropriate preset for a `WeatherParticleType`.
    pub fn preset_for(ptype: WeatherParticleType) -> Self {
        match ptype {
            WeatherParticleType::Rain => Self::rain(),
            WeatherParticleType::Snow => Self::snow(),
            WeatherParticleType::Sandstorm => Self::sandstorm(),
            WeatherParticleType::WindTrails => Self::wind_trails(),
        }
    }

    /// Scale the spawn rate and max particles by an intensity factor (0.0–1.0).
    ///
    /// Useful when driven by `WeatherSystem::get_rain_intensity()` etc.
    pub fn with_intensity(&self, intensity: f32) -> Self {
        let i = intensity.clamp(0.0, 1.0);
        Self {
            spawn_rate: (self.spawn_rate as f32 * i) as u32,
            max_particles: ((self.max_particles as f32 * i).max(1.0)) as u32,
            ..self.clone()
        }
    }

    /// Build `EmitterParams` suitable for `GpuParticleSystem::update()`.
    ///
    /// * `camera_pos` — current camera world position (spawn centre).
    /// * `wind_dir` — normalised wind direction (XZ plane).
    /// * `wind_strength` — wind magnitude (metres/sec).
    /// * `dt` — frame delta time.
    /// * `seed` — per-frame random seed.
    pub fn to_emitter_params(
        &self,
        camera_pos: Vec3,
        wind_dir: Vec3,
        wind_strength: f32,
        dt: f32,
        seed: u32,
    ) -> EmitterParams {
        // Combine base velocity with wind influence
        let wind_vel = wind_dir * wind_strength * self.wind_influence;
        let final_vel = self.velocity + wind_vel;

        let spawn_pos = Vec3::new(
            camera_pos.x,
            camera_pos.y + self.spawn_height_above_camera,
            camera_pos.z,
        );

        EmitterParams {
            position: [spawn_pos.x, spawn_pos.y, spawn_pos.z, self.spawn_radius],
            velocity: [final_vel.x, final_vel.y, final_vel.z, 0.0],
            emission_rate: self.spawn_rate as f32,
            lifetime: self.lifetime,
            velocity_randomness: self.velocity_randomness,
            delta_time: dt,
            gravity: [self.gravity[0], self.gravity[1], self.gravity[2], 0.0],
            particle_count: self.max_particles,
            max_particles: self.max_particles,
            random_seed: seed,
            _padding: 0,
        }
    }
}

// ── Integration with WeatherSystem ──────────────────────────────────────────

/// Maps the high-level `WeatherType` (environment.rs) to an optional GPU
/// particle config. Returns `None` for weather types that don't produce
/// visible precipitation (Clear, Cloudy, Fog).
pub fn config_for_weather(
    weather: crate::environment::WeatherType,
    rain_intensity: f32,
    snow_intensity: f32,
    wind_strength: f32,
) -> Option<WeatherParticleConfig> {
    use crate::environment::WeatherType;

    match weather {
        WeatherType::Rain => Some(WeatherParticleConfig::rain().with_intensity(rain_intensity)),
        WeatherType::Storm => {
            let mut cfg = WeatherParticleConfig::rain().with_intensity(rain_intensity);
            cfg.velocity.y = -25.0; // heavier droplets
            cfg.wind_influence = 1.0;
            Some(cfg)
        }
        WeatherType::Snow => Some(WeatherParticleConfig::snow().with_intensity(snow_intensity)),
        WeatherType::Sandstorm => {
            let mut cfg = WeatherParticleConfig::sandstorm();
            cfg.velocity.x = wind_strength * 15.0;
            Some(cfg)
        }
        WeatherType::Clear | WeatherType::Cloudy | WeatherType::Fog => None,
    }
}

/// Convenience: produce an `EmitterParams` directly from the current
/// `WeatherSystem` state for a single GPU dispatch.
///
/// Returns `None` if the weather doesn't require particles.
pub fn emitter_params_from_weather(
    weather: &crate::environment::WeatherSystem,
    camera_pos: Vec3,
    dt: f32,
    frame_seed: u32,
) -> Option<EmitterParams> {
    let cfg = config_for_weather(
        weather.current_weather(),
        weather.get_rain_intensity(),
        weather.get_snow_intensity(),
        weather.get_wind_strength(),
    )?;

    Some(cfg.to_emitter_params(
        camera_pos,
        weather.get_wind_direction(),
        weather.get_wind_strength(),
        dt,
        frame_seed,
    ))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rain_preset() {
        let cfg = WeatherParticleConfig::rain();
        assert_eq!(cfg.particle_type, WeatherParticleType::Rain);
        assert!(cfg.velocity.y < 0.0, "rain should fall");
        assert!(cfg.spawn_rate > 0);
        assert!(cfg.lifetime > 0.0);
    }

    #[test]
    fn test_snow_preset() {
        let cfg = WeatherParticleConfig::snow();
        assert_eq!(cfg.particle_type, WeatherParticleType::Snow);
        assert!(cfg.velocity.y < 0.0, "snow should fall");
        assert!(cfg.velocity.y > -5.0, "snow falls slowly");
        assert!(cfg.lifetime > cfg.velocity.y.abs() * 0.1, "snow should live long enough to be visible");
    }

    #[test]
    fn test_sandstorm_preset() {
        let cfg = WeatherParticleConfig::sandstorm();
        assert_eq!(cfg.particle_type, WeatherParticleType::Sandstorm);
        assert!(cfg.velocity.x > 0.0, "sandstorm blows horizontally");
        assert!(cfg.spawn_height_above_camera < 10.0, "sandstorm near ground");
    }

    #[test]
    fn test_wind_trails_preset() {
        let cfg = WeatherParticleConfig::wind_trails();
        assert_eq!(cfg.particle_type, WeatherParticleType::WindTrails);
        assert!(cfg.color[3] < 0.5, "wind trails are translucent");
    }

    #[test]
    fn test_preset_for() {
        let r = WeatherParticleConfig::preset_for(WeatherParticleType::Rain);
        assert_eq!(r.particle_type, WeatherParticleType::Rain);
        let s = WeatherParticleConfig::preset_for(WeatherParticleType::Snow);
        assert_eq!(s.particle_type, WeatherParticleType::Snow);
    }

    #[test]
    fn test_with_intensity_scales() {
        let full = WeatherParticleConfig::rain();
        let half = full.with_intensity(0.5);
        assert!(half.spawn_rate < full.spawn_rate);
        assert!(half.max_particles < full.max_particles);

        let zero = full.with_intensity(0.0);
        assert_eq!(zero.spawn_rate, 0);
        assert_eq!(zero.max_particles, 1); // clamped to at least 1
    }

    #[test]
    fn test_with_intensity_clamps() {
        let cfg = WeatherParticleConfig::rain();
        let over = cfg.with_intensity(5.0);
        // Should be clamped to 1.0 → same as the original
        assert_eq!(over.spawn_rate, cfg.spawn_rate);
    }

    #[test]
    fn test_to_emitter_params() {
        let cfg = WeatherParticleConfig::rain();
        let camera = Vec3::new(100.0, 50.0, 200.0);
        let wind_dir = Vec3::new(1.0, 0.0, 0.0);
        let wind_strength = 5.0;

        let params = cfg.to_emitter_params(camera, wind_dir, wind_strength, 0.016, 42);

        // Spawn position should be above camera
        assert!(params.position[1] > camera.y);
        // Spawn radius stored in position.w
        assert_eq!(params.position[3], cfg.spawn_radius);
        // Velocity should include wind contribution
        assert!(params.velocity[0] > cfg.velocity.x);
        assert_eq!(params.lifetime, cfg.lifetime);
        assert_eq!(params.delta_time, 0.016);
        assert_eq!(params.random_seed, 42);
    }

    #[test]
    fn test_to_emitter_params_no_wind() {
        let cfg = WeatherParticleConfig::snow();
        let params = cfg.to_emitter_params(Vec3::ZERO, Vec3::ZERO, 0.0, 0.016, 0);
        // Without wind, velocity should equal base velocity
        assert!((params.velocity[0] - cfg.velocity.x).abs() < 1e-6);
        assert!((params.velocity[1] - cfg.velocity.y).abs() < 1e-6);
    }

    #[test]
    fn test_config_for_weather_rain() {
        use crate::environment::WeatherType;
        let cfg = config_for_weather(WeatherType::Rain, 0.7, 0.0, 0.4);
        assert!(cfg.is_some());
        let c = cfg.unwrap();
        assert_eq!(c.particle_type, WeatherParticleType::Rain);
    }

    #[test]
    fn test_config_for_weather_clear_is_none() {
        use crate::environment::WeatherType;
        assert!(config_for_weather(WeatherType::Clear, 0.0, 0.0, 0.1).is_none());
        assert!(config_for_weather(WeatherType::Cloudy, 0.0, 0.0, 0.2).is_none());
        assert!(config_for_weather(WeatherType::Fog, 0.0, 0.0, 0.1).is_none());
    }

    #[test]
    fn test_config_for_weather_storm() {
        use crate::environment::WeatherType;
        let cfg = config_for_weather(WeatherType::Storm, 1.0, 0.0, 0.8).unwrap();
        assert!(cfg.velocity.y < -20.0, "storm rain falls faster");
        assert_eq!(cfg.wind_influence, 1.0);
    }

    #[test]
    fn test_config_for_weather_snow() {
        use crate::environment::WeatherType;
        let cfg = config_for_weather(WeatherType::Snow, 0.0, 0.8, 0.3).unwrap();
        assert_eq!(cfg.particle_type, WeatherParticleType::Snow);
    }

    #[test]
    fn test_config_for_weather_sandstorm() {
        use crate::environment::WeatherType;
        let cfg = config_for_weather(WeatherType::Sandstorm, 0.0, 0.0, 1.0).unwrap();
        assert_eq!(cfg.particle_type, WeatherParticleType::Sandstorm);
    }

    #[test]
    fn test_emitter_params_from_weather_clear() {
        let ws = crate::environment::WeatherSystem::new();
        let result = emitter_params_from_weather(&ws, Vec3::ZERO, 0.016, 0);
        assert!(result.is_none(), "clear weather should not produce particles");
    }

    #[test]
    fn test_emitter_params_size() {
        assert_eq!(std::mem::size_of::<EmitterParams>(), 80);
    }
}
