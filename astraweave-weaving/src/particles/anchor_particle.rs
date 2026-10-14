// Anchor Particle System - Decay and repair visual effects
//
// This module provides CPU-side particle spawning and update logic for Anchor VFX.
// Integrates with anchor_vfx.wgsl shader and Anchor.vfx_state enum.

use crate::sim_rng::{SimRng, SIM_SYSTEM_ANCHOR_PARTICLES};
use glam::Vec3;
use std::collections::VecDeque;

/// Unit direction from three dyadic draws. `sqrt` is correctly rounded, so the
/// bit pattern matches across OS. No `sin`/`cos`.
fn unit_direction(rng: &mut SimRng) -> Vec3 {
    let x = rng.signed_unit_f32();
    let y = rng.signed_unit_f32();
    let z = rng.signed_unit_f32();
    let len2 = x * x + y * y + z * z;
    if len2 > 1.0e-12 {
        let inv = 1.0 / len2.sqrt();
        Vec3::new(x * inv, y * inv, z * inv)
    } else {
        Vec3::Y
    }
}

/// Maximum particles per anchor (performance limit)
const MAX_PARTICLES_PER_ANCHOR: usize = 500;

/// Particle lifetime cap (auto-despawn after this duration)
#[allow(dead_code)]
const MAX_PARTICLE_LIFETIME: f32 = 3.0;

// ============================================================================
// Particle Type Definitions
// ============================================================================

/// Type of particle (determines behavior and appearance)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParticleType {
    /// Small blue sparks (Stable anchors, rare glitches)
    Spark,
    /// Yellow glitch effects (Unstable anchors, frequent)
    Glitch,
    /// Red reality tears (Critical anchors, expanding)
    Tear,
    /// Black void particles (Broken anchors, gravity pull)
    Void,
    /// Blue restoration wave (Repair animation)
    Restoration,
}

impl ParticleType {
    /// Get default lifetime for this particle type (seconds)
    pub fn default_lifetime(self) -> f32 {
        match self {
            ParticleType::Spark => 0.5,
            ParticleType::Glitch => 1.0,
            ParticleType::Tear => 2.0,
            ParticleType::Void => 3.0,
            ParticleType::Restoration => 1.5,
        }
    }

    /// Get particle color (RGB)
    pub fn color(self) -> Vec3 {
        match self {
            ParticleType::Spark => Vec3::new(0.2, 0.6, 1.0), // Bright blue
            ParticleType::Glitch => Vec3::new(0.9, 0.7, 0.2), // Yellow
            ParticleType::Tear => Vec3::new(1.0, 0.2, 0.1),  // Red
            ParticleType::Void => Vec3::new(0.1, 0.0, 0.2),  // Dark purple
            ParticleType::Restoration => Vec3::new(0.3, 0.8, 1.0), // Bright cyan
        }
    }

    /// Get particle size (world units)
    pub fn size(self) -> f32 {
        match self {
            ParticleType::Spark => 0.05,       // Small sparks
            ParticleType::Glitch => 0.1,       // Medium glitches
            ParticleType::Tear => 0.2,         // Large tears
            ParticleType::Void => 0.15,        // Medium void
            ParticleType::Restoration => 0.08, // Small restoration
        }
    }
}

// ============================================================================
// Particle Structure
// ============================================================================

/// Individual particle instance
#[derive(Debug, Clone)]
pub struct Particle {
    /// Particle type (determines behavior)
    pub particle_type: ParticleType,
    /// Current position (world space)
    pub position: Vec3,
    /// Current velocity (world units per second)
    pub velocity: Vec3,
    /// Time alive (seconds)
    pub age: f32,
    /// Maximum lifetime (seconds)
    pub lifetime: f32,
    /// Current size (world units, can grow/shrink)
    pub size: f32,
    /// Current alpha (0.0-1.0, for fade in/out)
    pub alpha: f32,
}

impl Particle {
    /// Create new particle at position with initial velocity
    pub fn new(particle_type: ParticleType, position: Vec3, velocity: Vec3) -> Self {
        Self {
            particle_type,
            position,
            velocity,
            age: 0.0,
            lifetime: particle_type.default_lifetime(),
            size: particle_type.size(),
            alpha: 1.0,
        }
    }

    /// Update particle (returns true if still alive, false if expired)
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.age += delta_time;

        // Check if expired
        if self.age >= self.lifetime {
            return false;
        }

        // Update position
        self.position += self.velocity * delta_time;

        // Apply type-specific behavior
        match self.particle_type {
            ParticleType::Spark => {
                // Sparks: Fade out linearly, decelerate
                self.alpha = 1.0 - (self.age / self.lifetime);
                self.velocity *= 0.95; // Slow down
            }
            ParticleType::Glitch => {
                // Glitches: Erratic motion (sine wave oscillation)
                let oscillation = (self.age * 10.0).sin() * 0.5;
                self.velocity.x += oscillation * delta_time;
                self.velocity.y += oscillation * 0.7 * delta_time;
                self.alpha = 1.0 - (self.age / self.lifetime).powi(2);
            }
            ParticleType::Tear => {
                // Tears: Expand over time, fade out
                let progress = self.age / self.lifetime;
                self.size = self.particle_type.size() * (1.0 + progress * 2.0); // 3× size at end
                self.alpha = 1.0 - progress;
            }
            ParticleType::Void => {
                // Void: Gravity pull toward anchor center, fade in then out
                let progress = self.age / self.lifetime;
                // Fade in (0.0-0.2), hold (0.2-0.8), fade out (0.8-1.0)
                if progress < 0.2 {
                    self.alpha = progress / 0.2;
                } else if progress > 0.8 {
                    self.alpha = (1.0 - progress) / 0.2;
                } else {
                    self.alpha = 1.0;
                }
                // Pull toward center (assumes anchor at origin, will be adjusted by system)
                self.velocity += -self.position.normalize() * 0.5 * delta_time;
            }
            ParticleType::Restoration => {
                // Restoration: Rise upward, fade out
                self.velocity.y += 1.0 * delta_time; // Float upward
                self.alpha = 1.0 - (self.age / self.lifetime);
            }
        }

        true // Still alive
    }

    /// Get current color with alpha applied
    pub fn color_with_alpha(&self) -> Vec3 {
        self.particle_type.color() * self.alpha
    }
}

// ============================================================================
// Anchor Particle Emitter
// ============================================================================

/// Particle emitter for a single anchor
pub struct AnchorParticleEmitter {
    /// Anchor ID (for identification)
    pub anchor_id: usize,
    /// Anchor world position (center of emission)
    pub anchor_position: Vec3,
    /// Current VFX state (0=Perfect, 1=Stable, 2=Unstable, 3=Critical, 4=Broken)
    pub vfx_state: u8,
    /// Is anchor currently being repaired? (triggers restoration particles)
    pub is_repairing: bool,
    /// Active particles
    particles: VecDeque<Particle>,
    /// Time accumulator for emission (fractional particles)
    emission_accumulator: f32,
}

impl AnchorParticleEmitter {
    /// Create new emitter for anchor
    pub fn new(anchor_id: usize, anchor_position: Vec3, vfx_state: u8) -> Self {
        Self {
            anchor_id,
            anchor_position,
            vfx_state,
            is_repairing: false,
            particles: VecDeque::with_capacity(MAX_PARTICLES_PER_ANCHOR),
            emission_accumulator: 0.0,
        }
    }

    /// Get emission rate (particles per second) for current VFX state
    pub fn emission_rate(&self) -> f32 {
        match self.vfx_state {
            0 => 0.0,   // Perfect - no particles
            1 => 5.0,   // Stable - rare glitches
            2 => 20.0,  // Unstable - frequent glitches
            3 => 50.0,  // Critical - reality tears
            4 => 100.0, // Broken - catastrophic
            _ => 0.0,   // Unknown state
        }
    }

    /// Get particle type for current VFX state
    fn particle_type_for_state(&self) -> ParticleType {
        match self.vfx_state {
            1 => ParticleType::Spark,  // Stable
            2 => ParticleType::Glitch, // Unstable
            3 => ParticleType::Tear,   // Critical
            4 => ParticleType::Void,   // Broken
            _ => ParticleType::Spark,  // Default (shouldn't happen for Perfect)
        }
    }

    /// Spawn a single particle at a seeded position on the anchor sphere.
    fn spawn_particle(&mut self, particle_type: ParticleType, rng: &mut SimRng) {
        // Check particle limit
        if self.particles.len() >= MAX_PARTICLES_PER_ANCHOR {
            // Remove oldest particle to make room
            self.particles.pop_front();
        }

        let direction = unit_direction(rng);
        let radius = 1.5;
        let position = self.anchor_position + direction * radius;

        // Outward speed in [0.5, 1.5).
        let speed = 0.5 + rng.unit_f32();
        let velocity = direction * speed;

        let particle = Particle::new(particle_type, position, velocity);
        self.particles.push_back(particle);
    }

    /// Spawn restoration particles (for repair animation)
    fn spawn_restoration_particles(&mut self, count: usize, rng: &mut SimRng) {
        for _ in 0..count {
            // Spawn at anchor base, rise upward. Offsets are in [-0.25, 0.25).
            let offset_x = rng.signed_unit_f32() * 0.25;
            let offset_z = rng.signed_unit_f32() * 0.25;
            let position = self.anchor_position + Vec3::new(offset_x, -0.5, offset_z);

            // Upward velocity with slight horizontal variation in [-0.1, 0.1).
            let vx = rng.signed_unit_f32() * 0.1;
            let vz = rng.signed_unit_f32() * 0.1;
            let velocity = Vec3::new(vx, 2.0, vz);

            let particle = Particle::new(ParticleType::Restoration, position, velocity);
            self.particles.push_back(particle);
        }
    }

    /// Update emitter (spawn new particles, update existing).
    ///
    /// `rng` is the sim stream for this tick. Callers reseed it from
    /// `(world_seed, tick, SIM_SYSTEM_ANCHOR_PARTICLES)` before the first emitter.
    pub fn update(&mut self, delta_time: f32, rng: &mut SimRng) {
        // Update existing particles (remove expired)
        self.particles.retain_mut(|p| p.update(delta_time));

        // Emit new decay particles based on VFX state
        let emission_rate = self.emission_rate();
        if emission_rate > 0.0 {
            // Accumulate fractional particles
            self.emission_accumulator += emission_rate * delta_time;

            // Spawn whole particles
            while self.emission_accumulator >= 1.0 {
                let particle_type = self.particle_type_for_state();
                self.spawn_particle(particle_type, rng);
                self.emission_accumulator -= 1.0;
            }
        }

        // Emit restoration particles if repairing
        if self.is_repairing {
            // 10 restoration particles per frame during repair (burst effect)
            self.spawn_restoration_particles(10, rng);
        }
    }

    /// Get active particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get all active particles (for rendering)
    pub fn particles(&self) -> &VecDeque<Particle> {
        &self.particles
    }

    /// Clear all particles (e.g., when anchor is destroyed)
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emission_accumulator = 0.0;
    }
}

// ============================================================================
// Particle System Manager (for all anchors)
// ============================================================================

/// Manages particle emitters for all anchors in scene
pub struct AnchorParticleSystem {
    emitters: Vec<AnchorParticleEmitter>,
    world_seed: u64,
    tick: u64,
}

impl AnchorParticleSystem {
    /// Create a particle system whose spawns come from `world_seed`.
    pub fn new(world_seed: u64) -> Self {
        Self {
            emitters: Vec::new(),
            world_seed,
            tick: 0,
        }
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Add emitter for anchor
    pub fn add_emitter(&mut self, anchor_id: usize, position: Vec3, vfx_state: u8) {
        let emitter = AnchorParticleEmitter::new(anchor_id, position, vfx_state);
        self.emitters.push(emitter);
    }

    /// Remove emitter for anchor
    pub fn remove_emitter(&mut self, anchor_id: usize) {
        self.emitters.retain(|e| e.anchor_id != anchor_id);
    }

    /// Update emitter state (position, VFX state, repair status)
    pub fn update_emitter(
        &mut self,
        anchor_id: usize,
        position: Vec3,
        vfx_state: u8,
        is_repairing: bool,
    ) {
        if let Some(emitter) = self.emitters.iter_mut().find(|e| e.anchor_id == anchor_id) {
            emitter.anchor_position = position;
            emitter.vfx_state = vfx_state;
            emitter.is_repairing = is_repairing;
        }
    }

    /// Update all emitters from one stream reseeded for this tick.
    pub fn update(&mut self, delta_time: f32) {
        self.tick = self.tick.wrapping_add(1);
        let mut rng = SimRng::new(self.world_seed, self.tick, SIM_SYSTEM_ANCHOR_PARTICLES);
        for emitter in &mut self.emitters {
            emitter.update(delta_time, &mut rng);
        }
    }

    /// Get total particle count across all emitters
    pub fn total_particle_count(&self) -> usize {
        self.emitters.iter().map(|e| e.particle_count()).sum()
    }

    /// Get all particles (for rendering)
    pub fn all_particles(&self) -> Vec<(&Particle, usize)> {
        let mut all_particles = Vec::new();
        for emitter in &self.emitters {
            for particle in emitter.particles() {
                all_particles.push((particle, emitter.anchor_id));
            }
        }
        all_particles
    }

    /// Clear all particles (e.g., scene transition)
    pub fn clear_all(&mut self) {
        for emitter in &mut self.emitters {
            emitter.clear();
        }
    }
}

impl Default for AnchorParticleSystem {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rng() -> SimRng {
        SimRng::new(1, 1, SIM_SYSTEM_ANCHOR_PARTICLES)
    }

    #[test]
    fn test_particle_creation() {
        let pos = Vec3::new(1.0, 2.0, 3.0);
        let vel = Vec3::new(0.1, 0.2, 0.3);
        let particle = Particle::new(ParticleType::Spark, pos, vel);

        assert_eq!(particle.particle_type, ParticleType::Spark);
        assert_eq!(particle.position, pos);
        assert_eq!(particle.velocity, vel);
        assert_eq!(particle.age, 0.0);
        assert_eq!(particle.lifetime, 0.5); // Spark default
        assert_eq!(particle.alpha, 1.0);
    }

    #[test]
    fn test_particle_update_spark() {
        let pos = Vec3::ZERO;
        let vel = Vec3::new(1.0, 0.0, 0.0);
        let mut particle = Particle::new(ParticleType::Spark, pos, vel);

        // Update 0.25s (halfway through 0.5s lifetime)
        let still_alive = particle.update(0.25);

        assert!(still_alive);
        assert_eq!(particle.age, 0.25);
        assert!(particle.alpha < 1.0); // Fading out
        assert!(particle.alpha > 0.0);
        assert!(particle.velocity.x < 1.0); // Decelerated
    }

    #[test]
    fn test_particle_expiry() {
        let pos = Vec3::ZERO;
        let vel = Vec3::ZERO;
        let mut particle = Particle::new(ParticleType::Spark, pos, vel);

        // Update beyond lifetime
        let still_alive = particle.update(1.0); // 0.5s lifetime + 0.5s

        assert!(!still_alive); // Should be expired
    }

    #[test]
    fn test_particle_tear_expansion() {
        let pos = Vec3::ZERO;
        let vel = Vec3::ZERO;
        let mut particle = Particle::new(ParticleType::Tear, pos, vel);

        let initial_size = particle.size;

        // Update halfway through lifetime
        particle.update(1.0); // 2.0s lifetime, so 50% progress

        assert!(particle.size > initial_size); // Should have expanded
    }

    #[test]
    fn test_emitter_creation() {
        let emitter = AnchorParticleEmitter::new(1, Vec3::ZERO, 2);

        assert_eq!(emitter.anchor_id, 1);
        assert_eq!(emitter.vfx_state, 2);
        assert_eq!(emitter.particle_count(), 0);
    }

    #[test]
    fn test_emission_rate() {
        let mut emitter = AnchorParticleEmitter::new(1, Vec3::ZERO, 1);
        assert_eq!(emitter.emission_rate(), 5.0); // Stable

        emitter.vfx_state = 2;
        assert_eq!(emitter.emission_rate(), 20.0); // Unstable

        emitter.vfx_state = 3;
        assert_eq!(emitter.emission_rate(), 50.0); // Critical

        emitter.vfx_state = 4;
        assert_eq!(emitter.emission_rate(), 100.0); // Broken
    }

    #[test]
    fn test_emitter_spawn_particles() {
        let mut emitter = AnchorParticleEmitter::new(1, Vec3::ZERO, 2); // Unstable

        // Update 1 second (20 particles expected @ 20 particles/sec)
        emitter.update(1.0, &mut test_rng());

        assert!(emitter.particle_count() > 15); // At least 15 particles (some randomness)
        assert!(emitter.particle_count() <= 25); // At most 25 particles
    }

    #[test]
    fn test_emitter_particle_limit() {
        let mut emitter = AnchorParticleEmitter::new(1, Vec3::ZERO, 4); // Broken (100/sec)

        // Update rapidly to hit limit (0.016s per frame @ 60 FPS, 10 frames)
        // Void particles live for 3s, so won't expire during this test
        for _ in 0..10 {
            emitter.update(0.016, &mut test_rng()); // 60 FPS frame time
        }

        // Should have spawned ~16 particles (100/sec × 0.016s × 10 frames)
        // But more importantly, test the limit by spawning many at once
        emitter.update(10.0, &mut test_rng()); // Spawn 1000 particles at once (100/sec × 10s)

        // Should cap at MAX_PARTICLES_PER_ANCHOR
        assert_eq!(emitter.particle_count(), MAX_PARTICLES_PER_ANCHOR);
    }

    #[test]
    fn test_restoration_particles() {
        let mut emitter = AnchorParticleEmitter::new(1, Vec3::ZERO, 2);
        emitter.is_repairing = true;

        // Update 1 frame (10 restoration particles per frame)
        emitter.update(0.016, &mut test_rng()); // ~60 FPS

        assert!(emitter.particle_count() >= 10); // At least 10 restoration particles
    }

    #[test]
    fn test_particle_system_manager() {
        let mut system = AnchorParticleSystem::new(1);

        // Add 3 emitters
        system.add_emitter(1, Vec3::ZERO, 1);
        system.add_emitter(2, Vec3::new(5.0, 0.0, 0.0), 2);
        system.add_emitter(3, Vec3::new(10.0, 0.0, 0.0), 3);

        // Update
        system.update(1.0);

        // Should have particles from all emitters
        assert!(system.total_particle_count() > 0);
    }

    #[test]
    fn test_particle_system_remove_emitter() {
        let mut system = AnchorParticleSystem::new(1);

        system.add_emitter(1, Vec3::ZERO, 2);
        system.add_emitter(2, Vec3::new(5.0, 0.0, 0.0), 2);

        // Spawn some particles
        system.update(1.0);
        let initial_count = system.total_particle_count();
        assert!(initial_count > 0);

        // Remove emitter 1
        system.remove_emitter(1);

        // Update again (only emitter 2 should emit)
        system.update(1.0);

        // Should have fewer emitters (particles may still be alive from emitter 1)
        assert_eq!(system.emitters.len(), 1);
    }

    #[test]
    fn test_particle_system_clear_all() {
        let mut system = AnchorParticleSystem::new(1);

        system.add_emitter(1, Vec3::ZERO, 4); // Broken (lots of particles)
        system.update(1.0);

        assert!(system.total_particle_count() > 0);

        system.clear_all();

        assert_eq!(system.total_particle_count(), 0);
    }

    #[test]
    fn particle_spawn_state_is_bit_identical_for_the_same_seed() {
        let run = || {
            let mut system = AnchorParticleSystem::new(0x5EED);
            system.add_emitter(7, Vec3::new(1.0, 2.0, 3.0), 2);
            system.update(0.1);
            system
                .all_particles()
                .into_iter()
                .map(|(p, id)| {
                    (
                        id,
                        p.position.x.to_bits(),
                        p.position.y.to_bits(),
                        p.position.z.to_bits(),
                        p.velocity.x.to_bits(),
                        p.velocity.y.to_bits(),
                        p.velocity.z.to_bits(),
                    )
                })
                .collect::<Vec<_>>()
        };
        let a = run();
        let b = run();
        assert!(!a.is_empty());
        assert_eq!(a, b);
    }
}
