//! Foam System for Water Surface Effects
//!
//! Provides dynamic foam generation at:
//! - Wave peaks (whitecaps)
//! - Shore/obstacle collisions
//! - Waterfall impact zones
//! - Boat/player wakes

use glam::{Vec2, Vec3};

/// Configuration for foam generation and rendering
#[derive(Clone, Debug, PartialEq)]
pub struct FoamConfig {
    /// Maximum number of foam particles
    pub max_particles: u32,
    /// Foam particle lifetime in seconds
    pub lifetime: f32,
    /// Foam spread rate (how fast it expands)
    pub spread_rate: f32,
    /// Foam fade rate (alpha decay per second)
    pub fade_rate: f32,
    /// Minimum wave steepness to generate whitecaps (0.0-1.0)
    pub whitecap_threshold: f32,
    /// Shore collision foam intensity multiplier
    pub shore_intensity: f32,
    /// Wake foam intensity multiplier
    pub wake_intensity: f32,
    /// Foam texture scale
    pub texture_scale: f32,
    /// Foam color tint
    pub color: Vec3,
    /// Foam opacity at spawn
    pub initial_opacity: f32,
}

impl Default for FoamConfig {
    fn default() -> Self {
        Self {
            max_particles: 10000,
            lifetime: 3.0,
            spread_rate: 0.5,
            fade_rate: 0.4,
            whitecap_threshold: 0.6,
            shore_intensity: 1.5,
            wake_intensity: 1.0,
            texture_scale: 2.0,
            color: Vec3::new(0.95, 0.98, 1.0),
            initial_opacity: 0.8,
        }
    }
}

impl FoamConfig {
    /// Create a calm water foam config (minimal foam)
    pub fn calm() -> Self {
        Self {
            max_particles: 2000,
            lifetime: 2.0,
            whitecap_threshold: 0.9,
            shore_intensity: 0.5,
            wake_intensity: 0.5,
            initial_opacity: 0.5,
            ..Default::default()
        }
    }

    /// Create a stormy water foam config (heavy foam)
    pub fn stormy() -> Self {
        Self {
            max_particles: 20000,
            lifetime: 4.0,
            whitecap_threshold: 0.3,
            shore_intensity: 2.5,
            wake_intensity: 2.0,
            initial_opacity: 1.0,
            spread_rate: 0.8,
            ..Default::default()
        }
    }

    /// Create a river rapids foam config
    pub fn rapids() -> Self {
        Self {
            max_particles: 15000,
            lifetime: 1.5,
            whitecap_threshold: 0.4,
            shore_intensity: 2.0,
            wake_intensity: 1.5,
            spread_rate: 1.0,
            fade_rate: 0.6,
            initial_opacity: 0.9,
            ..Default::default()
        }
    }
}

/// A single foam particle
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FoamParticle {
    /// World position (x, y on water surface, z is height offset)
    pub position: Vec3,
    /// Velocity for movement
    pub velocity: Vec2,
    /// Current size (grows over lifetime)
    pub size: f32,
    /// Remaining lifetime in seconds
    pub lifetime: f32,
    /// Current opacity (0-1)
    pub opacity: f32,
    /// Rotation angle for texture
    pub rotation: f32,
    /// Source type (for debugging/different rendering)
    pub source: FoamSource,
}

/// Source of foam generation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum FoamSource {
    /// Whitecap from steep wave
    Whitecap = 0,
    /// Shore/obstacle collision
    Shore = 1,
    /// Waterfall impact
    Waterfall = 2,
    /// Object wake (boat, player)
    Wake = 3,
    /// Rapids/turbulence
    Rapids = 4,
}

/// GPU-compatible foam particle data
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct GpuFoamParticle {
    /// Position (xyz) + size (w)
    pub position_size: [f32; 4],
    /// Velocity (xy) + rotation (z) + opacity (w)
    pub velocity_rot_opacity: [f32; 4],
}

impl From<&FoamParticle> for GpuFoamParticle {
    fn from(p: &FoamParticle) -> Self {
        Self {
            position_size: [p.position.x, p.position.y, p.position.z, p.size],
            velocity_rot_opacity: [p.velocity.x, p.velocity.y, p.rotation, p.opacity],
        }
    }
}

/// Manages foam particle simulation and generation
#[derive(Debug)]
pub struct FoamSystem {
    /// Active foam particles
    particles: Vec<FoamParticle>,
    /// Configuration
    config: FoamConfig,
    /// Time accumulator for spawning (reserved for frame-rate independent spawning)
    #[allow(dead_code)]
    spawn_accumulator: f32,
    /// Random seed for variation
    seed: u32,
}

impl FoamSystem {
    /// Create a new foam system
    pub fn new(config: FoamConfig) -> Self {
        Self {
            particles: Vec::with_capacity(config.max_particles as usize),
            config,
            spawn_accumulator: 0.0,
            seed: 12345,
        }
    }

    /// Get current particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get configuration
    pub fn config(&self) -> &FoamConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: FoamConfig) {
        self.config = config;
    }

    /// Simple pseudo-random number generator
    fn next_random(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.seed >> 16) & 0x7FFF) as f32 / 32767.0
    }

    /// Spawn foam at a specific location
    pub fn spawn_foam(&mut self, position: Vec3, velocity: Vec2, source: FoamSource) {
        if self.particles.len() >= self.config.max_particles as usize {
            return;
        }

        let intensity = match source {
            FoamSource::Shore => self.config.shore_intensity,
            FoamSource::Wake => self.config.wake_intensity,
            FoamSource::Waterfall => self.config.shore_intensity * 1.5,
            _ => 1.0,
        };

        let particle = FoamParticle {
            position,
            velocity,
            size: 0.1 + self.next_random() * 0.1,
            lifetime: self.config.lifetime * (0.8 + self.next_random() * 0.4),
            opacity: self.config.initial_opacity * intensity.min(1.0),
            rotation: self.next_random() * std::f32::consts::TAU,
            source,
        };

        self.particles.push(particle);
    }

    /// Spawn multiple foam particles in a burst pattern
    pub fn spawn_burst(&mut self, center: Vec3, count: u32, radius: f32, source: FoamSource) {
        for _ in 0..count {
            if self.particles.len() >= self.config.max_particles as usize {
                break;
            }

            let angle = self.next_random() * std::f32::consts::TAU;
            let dist = self.next_random() * radius;
            let offset = Vec3::new(angle.cos() * dist, angle.sin() * dist, 0.0);

            let velocity = Vec2::new(
                (self.next_random() - 0.5) * 0.5,
                (self.next_random() - 0.5) * 0.5,
            );

            self.spawn_foam(center + offset, velocity, source);
        }
    }

    /// Generate whitecaps based on wave steepness
    pub fn generate_whitecaps(&mut self, wave_positions: &[(Vec3, f32)]) {
        for &(pos, steepness) in wave_positions {
            if steepness > self.config.whitecap_threshold {
                let intensity = (steepness - self.config.whitecap_threshold)
                    / (1.0 - self.config.whitecap_threshold);
                let count = (intensity * 5.0) as u32 + 1;
                self.spawn_burst(pos, count, 0.3, FoamSource::Whitecap);
            }
        }
    }

    /// Generate shore collision foam
    pub fn generate_shore_foam(&mut self, collision_points: &[Vec3], water_velocity: Vec2) {
        for &pos in collision_points {
            let splash_velocity = -water_velocity * 0.3;
            self.spawn_burst(pos, 8, 0.5, FoamSource::Shore);

            // Add some directed particles
            for _ in 0..4 {
                let rand_mult = 0.5 + self.next_random();
                self.spawn_foam(pos, splash_velocity * rand_mult, FoamSource::Shore);
            }
        }
    }

    /// Generate wake foam behind a moving object
    pub fn generate_wake(&mut self, object_pos: Vec3, object_velocity: Vec2, object_width: f32) {
        let speed = object_velocity.length();
        if speed < 0.1 {
            return;
        }

        // Normalize velocity for direction
        let dir = object_velocity / speed;
        let perp = Vec2::new(-dir.y, dir.x);

        // Spawn foam particles in a V-wake pattern
        let wake_angle = 0.3; // Wake spread angle
        let particle_count = ((speed * 3.0) as u32).min(10);

        for i in 0..particle_count {
            let t = i as f32 / particle_count as f32;

            // Left wake arm
            let left_offset = -dir * (t * 2.0) + perp * (t * object_width * wake_angle);
            let left_pos = object_pos + Vec3::new(left_offset.x, left_offset.y, 0.0);
            let left_vel = (-dir + perp * 0.3) * speed * 0.2;
            self.spawn_foam(left_pos, left_vel, FoamSource::Wake);

            // Right wake arm
            let right_offset = -dir * (t * 2.0) - perp * (t * object_width * wake_angle);
            let right_pos = object_pos + Vec3::new(right_offset.x, right_offset.y, 0.0);
            let right_vel = (-dir - perp * 0.3) * speed * 0.2;
            self.spawn_foam(right_pos, right_vel, FoamSource::Wake);
        }

        // Center turbulence
        self.spawn_burst(
            object_pos - Vec3::new(dir.x, dir.y, 0.0),
            3,
            0.2,
            FoamSource::Wake,
        );
    }

    /// Update foam simulation
    pub fn update(&mut self, dt: f32) {
        let spread_rate = self.config.spread_rate;
        let fade_rate = self.config.fade_rate;

        // Update all particles
        self.particles.retain_mut(|p| {
            // Decrease lifetime
            p.lifetime -= dt;
            if p.lifetime <= 0.0 {
                return false;
            }

            // Move particle
            p.position.x += p.velocity.x * dt;
            p.position.y += p.velocity.y * dt;

            // Slow down velocity (drag)
            p.velocity *= 1.0 - dt * 2.0;

            // Expand size
            p.size += spread_rate * dt;

            // Fade opacity
            p.opacity -= fade_rate * dt;
            if p.opacity <= 0.0 {
                return false;
            }

            // Rotate slightly
            p.rotation += dt * 0.5;

            true
        });
    }

    /// Get particles for GPU rendering
    pub fn get_gpu_particles(&self) -> Vec<GpuFoamParticle> {
        self.particles.iter().map(GpuFoamParticle::from).collect()
    }

    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Get particles slice for inspection
    pub fn particles(&self) -> &[FoamParticle] {
        &self.particles
    }
}

/// Foam trail system for persistent foam paths (boat wakes, etc.)
#[derive(Debug)]
pub struct FoamTrail {
    /// Trail points with age
    points: Vec<FoamTrailPoint>,
    /// Maximum trail length
    max_points: usize,
    /// Trail width
    width: f32,
    /// Fade time in seconds
    fade_time: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct FoamTrailPoint {
    pub position: Vec3,
    pub width: f32,
    pub age: f32,
}

impl FoamTrail {
    /// Create a new foam trail
    pub fn new(max_points: usize, width: f32, fade_time: f32) -> Self {
        Self {
            points: Vec::with_capacity(max_points),
            max_points,
            width,
            fade_time,
        }
    }

    /// Add a point to the trail
    pub fn add_point(&mut self, position: Vec3) {
        if self.points.len() >= self.max_points {
            self.points.remove(0);
        }

        self.points.push(FoamTrailPoint {
            position,
            width: self.width,
            age: 0.0,
        });
    }

    /// Update trail ages
    pub fn update(&mut self, dt: f32) {
        self.points.retain_mut(|p| {
            p.age += dt;
            p.age < self.fade_time
        });
    }

    /// Get trail points
    pub fn points(&self) -> &[FoamTrailPoint] {
        &self.points
    }

    /// Get opacity for a point based on age
    pub fn get_opacity(&self, point: &FoamTrailPoint) -> f32 {
        1.0 - (point.age / self.fade_time)
    }

    /// Clear the trail
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foam_config_default() {
        let config = FoamConfig::default();
        assert_eq!(config.max_particles, 10000);
        assert!(config.lifetime > 0.0);
        assert!(config.whitecap_threshold > 0.0 && config.whitecap_threshold < 1.0);
    }

    #[test]
    fn test_foam_config_presets() {
        let calm = FoamConfig::calm();
        let stormy = FoamConfig::stormy();
        let rapids = FoamConfig::rapids();

        // Stormy should have more particles than calm
        assert!(stormy.max_particles > calm.max_particles);

        // Stormy should have lower whitecap threshold (more whitecaps)
        assert!(stormy.whitecap_threshold < calm.whitecap_threshold);

        // Rapids should have short lifetime
        assert!(rapids.lifetime < stormy.lifetime);
    }

    #[test]
    fn test_foam_system_creation() {
        let system = FoamSystem::new(FoamConfig::default());
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_foam_spawn() {
        let mut system = FoamSystem::new(FoamConfig::default());

        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Whitecap);
        assert_eq!(system.particle_count(), 1);

        system.spawn_foam(Vec3::ONE, Vec2::ONE, FoamSource::Shore);
        assert_eq!(system.particle_count(), 2);
    }

    #[test]
    fn test_foam_spawn_burst() {
        let mut system = FoamSystem::new(FoamConfig::default());

        system.spawn_burst(Vec3::ZERO, 10, 1.0, FoamSource::Waterfall);
        assert_eq!(system.particle_count(), 10);
    }

    #[test]
    fn test_foam_max_particles() {
        let config = FoamConfig {
            max_particles: 5,
            ..Default::default()
        };
        let mut system = FoamSystem::new(config);

        // Try to spawn more than max
        for _ in 0..10 {
            system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Whitecap);
        }

        assert_eq!(system.particle_count(), 5);
    }

    #[test]
    fn test_foam_update_lifetime() {
        let config = FoamConfig {
            lifetime: 1.0,
            fade_rate: 0.0, // Don't fade, only lifetime matters
            ..Default::default()
        };
        let mut system = FoamSystem::new(config);

        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Whitecap);
        assert_eq!(system.particle_count(), 1);

        // Update past lifetime
        system.update(2.0);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_foam_update_fade() {
        let config = FoamConfig {
            lifetime: 10.0, // Long lifetime
            fade_rate: 1.0, // Fade in 1 second
            initial_opacity: 1.0,
            ..Default::default()
        };
        let mut system = FoamSystem::new(config);

        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Whitecap);

        // Update to fade out
        system.update(1.5);
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_foam_wake_generation() {
        let mut system = FoamSystem::new(FoamConfig::default());

        // Generate wake behind moving object
        system.generate_wake(Vec3::ZERO, Vec2::new(1.0, 0.0), 2.0);

        // Should have spawned particles
        assert!(system.particle_count() > 0);
    }

    #[test]
    fn test_foam_whitecap_generation() {
        let config = FoamConfig {
            whitecap_threshold: 0.5,
            ..Default::default()
        };
        let mut system = FoamSystem::new(config);

        // Wave below threshold - no foam
        system.generate_whitecaps(&[(Vec3::ZERO, 0.3)]);
        assert_eq!(system.particle_count(), 0);

        // Wave above threshold - foam generated
        system.generate_whitecaps(&[(Vec3::ONE, 0.8)]);
        assert!(system.particle_count() > 0);
    }

    #[test]
    fn test_gpu_particle_conversion() {
        let particle = FoamParticle {
            position: Vec3::new(1.0, 2.0, 3.0),
            velocity: Vec2::new(0.5, 0.5),
            size: 0.2,
            lifetime: 1.0,
            opacity: 0.8,
            rotation: 0.5,
            source: FoamSource::Whitecap,
        };

        let gpu: GpuFoamParticle = (&particle).into();

        assert_eq!(gpu.position_size[0], 1.0);
        assert_eq!(gpu.position_size[1], 2.0);
        assert_eq!(gpu.position_size[2], 3.0);
        assert_eq!(gpu.position_size[3], 0.2);
        assert_eq!(gpu.velocity_rot_opacity[3], 0.8);
    }

    #[test]
    fn test_foam_trail() {
        let mut trail = FoamTrail::new(10, 1.0, 2.0);

        trail.add_point(Vec3::ZERO);
        trail.add_point(Vec3::ONE);
        assert_eq!(trail.points().len(), 2);

        // Update ages
        trail.update(1.0);
        assert_eq!(trail.points().len(), 2);

        // Age past fade time
        trail.update(1.5);
        assert_eq!(trail.points().len(), 0);
    }

    #[test]
    fn test_foam_trail_max_points() {
        let mut trail = FoamTrail::new(3, 1.0, 10.0);

        for i in 0..5 {
            trail.add_point(Vec3::new(i as f32, 0.0, 0.0));
        }

        assert_eq!(trail.points().len(), 3);
        // Oldest points should be removed
        assert_eq!(trail.points()[0].position.x, 2.0);
    }

    #[test]
    fn test_foam_source_values() {
        assert_eq!(FoamSource::Whitecap as u8, 0);
        assert_eq!(FoamSource::Shore as u8, 1);
        assert_eq!(FoamSource::Waterfall as u8, 2);
        assert_eq!(FoamSource::Wake as u8, 3);
        assert_eq!(FoamSource::Rapids as u8, 4);
    }

    #[test]
    fn test_foam_clear() {
        let mut system = FoamSystem::new(FoamConfig::default());

        system.spawn_burst(Vec3::ZERO, 50, 1.0, FoamSource::Shore);
        assert!(system.particle_count() > 0);

        system.clear();
        assert_eq!(system.particle_count(), 0);
    }

    #[test]
    fn test_gpu_foam_particle_size() {
        // GpuFoamParticle: 2 * [f32; 4] = 32 bytes (wgpu aligned)
        assert_eq!(std::mem::size_of::<GpuFoamParticle>(), 32);
    }

    // =========================================================================
    // Mutation-killing tests
    // =========================================================================

    #[test]
    fn test_calm_all_fields() {
        let c = FoamConfig::calm();
        assert_eq!(c.max_particles, 2000);
        assert!((c.lifetime - 2.0).abs() < 1e-6);
        assert!((c.whitecap_threshold - 0.9).abs() < 1e-6);
        assert!((c.shore_intensity - 0.5).abs() < 1e-6);
        assert!((c.wake_intensity - 0.5).abs() < 1e-6);
        assert!((c.initial_opacity - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_stormy_all_fields() {
        let c = FoamConfig::stormy();
        assert_eq!(c.max_particles, 20000);
        assert!((c.lifetime - 4.0).abs() < 1e-6);
        assert!((c.whitecap_threshold - 0.3).abs() < 1e-6);
        assert!((c.shore_intensity - 2.5).abs() < 1e-6);
        assert!((c.wake_intensity - 2.0).abs() < 1e-6);
        assert!((c.initial_opacity - 1.0).abs() < 1e-6);
        assert!((c.spread_rate - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_rapids_all_fields() {
        let c = FoamConfig::rapids();
        assert_eq!(c.max_particles, 15000);
        assert!((c.lifetime - 1.5).abs() < 1e-6);
        assert!((c.whitecap_threshold - 0.4).abs() < 1e-6);
        assert!((c.shore_intensity - 2.0).abs() < 1e-6);
        assert!((c.wake_intensity - 1.5).abs() < 1e-6);
        assert!((c.spread_rate - 1.0).abs() < 1e-6);
        assert!((c.fade_rate - 0.6).abs() < 1e-6);
        assert!((c.initial_opacity - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_config_getter_returns_set_config() {
        let cfg = FoamConfig::calm();
        let system = FoamSystem::new(cfg.clone());
        assert_eq!(system.config().max_particles, 2000);
        assert!((system.config().lifetime - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_set_config_updates() {
        let mut system = FoamSystem::new(FoamConfig::default());
        assert_eq!(system.config().max_particles, 10000);
        system.set_config(FoamConfig::calm());
        assert_eq!(system.config().max_particles, 2000);
    }

    #[test]
    fn test_next_random_range_and_determinism() {
        let mut system = FoamSystem::new(FoamConfig::default());
        let v1 = system.next_random();
        let v2 = system.next_random();
        let v3 = system.next_random();
        // Values must be in [0, 1)
        assert!(v1 >= 0.0 && v1 < 1.0, "v1={}", v1);
        assert!(v2 >= 0.0 && v2 < 1.0, "v2={}", v2);
        assert!(v3 >= 0.0 && v3 < 1.0, "v3={}", v3);
        // Not all the same
        assert!(v1 != v2 || v2 != v3, "RNG stuck: {} {} {}", v1, v2, v3);

        // Deterministic: same seed → same values
        let mut system2 = FoamSystem::new(FoamConfig::default());
        assert!((system2.next_random() - v1).abs() < 1e-6);
        assert!((system2.next_random() - v2).abs() < 1e-6);
    }

    #[test]
    fn test_spawn_foam_source_intensity_shore() {
        // shore_intensity < 1.0 ensures Shore arm makes a difference vs default (1.0)
        let cfg = FoamConfig {
            shore_intensity: 0.5,
            initial_opacity: 0.8,
            ..Default::default()
        };
        let mut system = FoamSystem::new(cfg);
        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Shore);
        let p = &system.particles()[0];
        // intensity = shore_intensity = 0.5, .min(1.0) = 0.5
        // opacity = initial_opacity * 0.5 = 0.4
        assert!(
            (p.opacity - 0.4).abs() < 1e-4,
            "shore opacity={}",
            p.opacity
        );
    }

    #[test]
    fn test_spawn_foam_source_intensity_wake() {
        let cfg = FoamConfig {
            wake_intensity: 0.3,
            initial_opacity: 0.8,
            ..Default::default()
        };
        let mut system = FoamSystem::new(cfg);
        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Wake);
        let p = &system.particles()[0];
        // intensity = wake_intensity = 0.3, opacity = 0.8 * 0.3 = 0.24
        assert!(
            (p.opacity - 0.24).abs() < 1e-4,
            "wake opacity={}",
            p.opacity
        );
    }

    #[test]
    fn test_spawn_foam_source_intensity_waterfall() {
        let cfg = FoamConfig {
            shore_intensity: 0.4,
            initial_opacity: 1.0,
            ..Default::default()
        };
        let mut system = FoamSystem::new(cfg);
        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Waterfall);
        let p = &system.particles()[0];
        // intensity = shore_intensity * 1.5 = 0.6, .min(1.0) = 0.6
        // opacity = 1.0 * 0.6 = 0.6
        assert!(
            (p.opacity - 0.6).abs() < 1e-4,
            "waterfall opacity={}",
            p.opacity
        );
    }

    #[test]
    fn test_spawn_foam_size_has_random_component() {
        let mut system = FoamSystem::new(FoamConfig::default());
        system.spawn_foam(Vec3::ZERO, Vec2::ZERO, FoamSource::Whitecap);
        let p = &system.particles()[0];
        // size = 0.1 + next_random() * 0.1 → should be in [0.1, 0.2)
        assert!(p.size >= 0.1, "size too small: {}", p.size);
        assert!(p.size < 0.2 + 1e-6, "size too large: {}", p.size);
    }
}
