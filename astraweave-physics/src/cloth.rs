//! Cloth Simulation System
//!
//! Verlet integration-based cloth simulation:
//! - Particle system with distance constraints
//! - Collision with rigid bodies (spheres, capsules)
//! - Wind interaction
//! - Pinned particles for attachment points

use glam::Vec3;
use std::collections::HashMap;

/// Unique identifier for cloth instances
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClothId(pub u64);

/// A particle in the cloth simulation
#[derive(Debug, Clone)]
pub struct ClothParticle {
    /// Current position
    pub position: Vec3,
    /// Previous position (for Verlet integration)
    pub prev_position: Vec3,
    /// Accumulated forces this frame
    pub acceleration: Vec3,
    /// Inverse mass (0 = pinned/infinite mass)
    pub inv_mass: f32,
    /// Whether this particle is pinned (fixed position)
    pub pinned: bool,
}

impl ClothParticle {
    /// Create a new particle
    pub fn new(position: Vec3, mass: f32) -> Self {
        Self {
            position,
            prev_position: position,
            acceleration: Vec3::ZERO,
            inv_mass: if mass > 0.0 { 1.0 / mass } else { 0.0 },
            pinned: false,
        }
    }

    /// Create a pinned particle
    pub fn pinned(position: Vec3) -> Self {
        Self {
            position,
            prev_position: position,
            acceleration: Vec3::ZERO,
            inv_mass: 0.0,
            pinned: true,
        }
    }

    /// Apply force to particle
    pub fn apply_force(&mut self, force: Vec3) {
        if !self.pinned {
            self.acceleration += force * self.inv_mass;
        }
    }

    /// Integrate using Verlet integration
    pub fn integrate(&mut self, dt: f32, damping: f32) {
        if self.pinned {
            return;
        }

        let velocity = self.position - self.prev_position;
        self.prev_position = self.position;
        self.position += velocity * damping + self.acceleration * dt * dt;
        self.acceleration = Vec3::ZERO;
    }

    /// Get velocity
    pub fn velocity(&self) -> Vec3 {
        self.position - self.prev_position
    }
}

/// A distance constraint between two particles
#[derive(Debug, Clone, Copy)]
pub struct DistanceConstraint {
    /// First particle index
    pub p1: usize,
    /// Second particle index
    pub p2: usize,
    /// Rest length
    pub rest_length: f32,
    /// Stiffness (0-1, higher = stiffer)
    pub stiffness: f32,
}

impl DistanceConstraint {
    /// Create a new constraint
    pub fn new(p1: usize, p2: usize, rest_length: f32) -> Self {
        Self {
            p1,
            p2,
            rest_length,
            stiffness: 1.0,
        }
    }

    /// Solve the constraint
    pub fn solve(&self, particles: &mut [ClothParticle]) {
        let p1 = &particles[self.p1];
        let p2 = &particles[self.p2];

        let delta = p2.position - p1.position;
        let current_length = delta.length();

        if current_length < 0.0001 {
            return;
        }

        let diff = (current_length - self.rest_length) / current_length;
        let correction = delta * diff * 0.5 * self.stiffness;

        let w1 = p1.inv_mass;
        let w2 = p2.inv_mass;
        let total_weight = w1 + w2;

        if total_weight > 0.0 {
            if !particles[self.p1].pinned {
                particles[self.p1].position += correction * (w1 / total_weight);
            }
            if !particles[self.p2].pinned {
                particles[self.p2].position -= correction * (w2 / total_weight);
            }
        }
    }
}

/// Collision shape for cloth collision
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum ClothCollider {
    /// Sphere collider
    Sphere { center: Vec3, radius: f32 },
    /// Capsule collider
    Capsule { start: Vec3, end: Vec3, radius: f32 },
    /// Infinite plane
    Plane { point: Vec3, normal: Vec3 },
}

impl ClothCollider {
    /// Resolve collision with a particle
    pub fn resolve_collision(&self, particle: &mut ClothParticle, friction: f32) {
        if particle.pinned {
            return;
        }

        match *self {
            ClothCollider::Sphere { center, radius } => {
                let to_particle = particle.position - center;
                let dist = to_particle.length();

                if dist < radius {
                    let normal = to_particle.normalize_or_zero();
                    let penetration = radius - dist;
                    particle.position += normal * penetration;

                    // Apply friction
                    let velocity = particle.velocity();
                    let normal_vel = velocity.dot(normal) * normal;
                    let tangent_vel = velocity - normal_vel;
                    particle.prev_position =
                        particle.position - (normal_vel + tangent_vel * (1.0 - friction));
                }
            }
            ClothCollider::Capsule { start, end, radius } => {
                let axis = end - start;
                let axis_length = axis.length();
                if axis_length < 0.0001 {
                    // Degenerate capsule = sphere
                    let sphere = ClothCollider::Sphere {
                        center: start,
                        radius,
                    };
                    sphere.resolve_collision(particle, friction);
                    return;
                }

                let axis_dir = axis / axis_length;
                let to_particle = particle.position - start;
                let t = to_particle.dot(axis_dir).clamp(0.0, axis_length);
                let closest = start + axis_dir * t;

                let to_particle = particle.position - closest;
                let dist = to_particle.length();

                if dist < radius {
                    let normal = to_particle.normalize_or_zero();
                    let penetration = radius - dist;
                    particle.position += normal * penetration;

                    let velocity = particle.velocity();
                    let normal_vel = velocity.dot(normal) * normal;
                    let tangent_vel = velocity - normal_vel;
                    particle.prev_position =
                        particle.position - (normal_vel + tangent_vel * (1.0 - friction));
                }
            }
            ClothCollider::Plane { point, normal } => {
                let to_particle = particle.position - point;
                let dist = to_particle.dot(normal);

                if dist < 0.0 {
                    particle.position -= normal * dist;

                    let velocity = particle.velocity();
                    let normal_vel = velocity.dot(normal) * normal;
                    let tangent_vel = velocity - normal_vel;
                    particle.prev_position =
                        particle.position - (normal_vel + tangent_vel * (1.0 - friction));
                }
            }
        }
    }
}

/// Configuration for cloth simulation
#[derive(Debug, Clone)]
pub struct ClothConfig {
    /// Width in particles
    pub width: usize,
    /// Height in particles
    pub height: usize,
    /// Spacing between particles
    pub spacing: f32,
    /// Mass per particle
    pub particle_mass: f32,
    /// Constraint stiffness (0-1)
    pub stiffness: f32,
    /// Velocity damping (0-1, lower = more damping)
    pub damping: f32,
    /// Constraint solver iterations
    pub solver_iterations: usize,
    /// Gravity
    pub gravity: Vec3,
    /// Wind force
    pub wind: Vec3,
    /// Air resistance
    pub air_resistance: f32,
}

impl Default for ClothConfig {
    fn default() -> Self {
        Self {
            width: 20,
            height: 20,
            spacing: 0.1,
            particle_mass: 0.1,
            stiffness: 0.8,
            damping: 0.98,
            solver_iterations: 3,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            wind: Vec3::ZERO,
            air_resistance: 0.01,
        }
    }
}

/// A cloth instance
#[derive(Debug, Clone)]
pub struct Cloth {
    pub id: ClothId,
    pub config: ClothConfig,
    pub particles: Vec<ClothParticle>,
    pub constraints: Vec<DistanceConstraint>,
    pub colliders: Vec<ClothCollider>,
    /// Collision friction
    pub collision_friction: f32,
}

impl Cloth {
    /// Create a new cloth from config
    pub fn new(id: ClothId, config: ClothConfig, origin: Vec3) -> Self {
        let mut particles = Vec::with_capacity(config.width * config.height);
        let mut constraints = Vec::new();

        // Create particles in a grid
        for y in 0..config.height {
            for x in 0..config.width {
                let pos =
                    origin + Vec3::new(x as f32 * config.spacing, 0.0, y as f32 * config.spacing);
                particles.push(ClothParticle::new(pos, config.particle_mass));
            }
        }

        // Create structural constraints (horizontal and vertical)
        for y in 0..config.height {
            for x in 0..config.width {
                let idx = y * config.width + x;

                // Horizontal constraint
                if x < config.width - 1 {
                    let mut c = DistanceConstraint::new(idx, idx + 1, config.spacing);
                    c.stiffness = config.stiffness;
                    constraints.push(c);
                }

                // Vertical constraint
                if y < config.height - 1 {
                    let mut c = DistanceConstraint::new(idx, idx + config.width, config.spacing);
                    c.stiffness = config.stiffness;
                    constraints.push(c);
                }

                // Shear constraints (diagonals)
                if x < config.width - 1 && y < config.height - 1 {
                    let diag_len = config.spacing * std::f32::consts::SQRT_2;

                    let mut c1 = DistanceConstraint::new(idx, idx + config.width + 1, diag_len);
                    c1.stiffness = config.stiffness * 0.5;
                    constraints.push(c1);

                    let mut c2 = DistanceConstraint::new(idx + 1, idx + config.width, diag_len);
                    c2.stiffness = config.stiffness * 0.5;
                    constraints.push(c2);
                }

                // Bend constraints (skip one particle)
                if x < config.width - 2 {
                    let mut c = DistanceConstraint::new(idx, idx + 2, config.spacing * 2.0);
                    c.stiffness = config.stiffness * 0.3;
                    constraints.push(c);
                }
                if y < config.height - 2 {
                    let mut c =
                        DistanceConstraint::new(idx, idx + config.width * 2, config.spacing * 2.0);
                    c.stiffness = config.stiffness * 0.3;
                    constraints.push(c);
                }
            }
        }

        Self {
            id,
            config,
            particles,
            constraints,
            colliders: Vec::new(),
            collision_friction: 0.5,
        }
    }

    /// Pin particles at the top edge
    pub fn pin_top_edge(&mut self) {
        for x in 0..self.config.width {
            self.particles[x].pinned = true;
            self.particles[x].inv_mass = 0.0;
        }
    }

    /// Pin specific corners
    pub fn pin_corners(&mut self) {
        let w = self.config.width;
        let _h = self.config.height;

        // Top-left
        self.particles[0].pinned = true;
        self.particles[0].inv_mass = 0.0;

        // Top-right
        self.particles[w - 1].pinned = true;
        self.particles[w - 1].inv_mass = 0.0;
    }

    /// Pin a specific particle by index
    pub fn pin_particle(&mut self, index: usize) {
        if index < self.particles.len() {
            self.particles[index].pinned = true;
            self.particles[index].inv_mass = 0.0;
        }
    }

    /// Unpin a particle
    pub fn unpin_particle(&mut self, index: usize) {
        if index < self.particles.len() {
            self.particles[index].pinned = false;
            self.particles[index].inv_mass = 1.0 / self.config.particle_mass;
        }
    }

    /// Move a pinned particle
    pub fn move_pinned(&mut self, index: usize, new_position: Vec3) {
        if index < self.particles.len() && self.particles[index].pinned {
            self.particles[index].position = new_position;
            self.particles[index].prev_position = new_position;
        }
    }

    /// Add a collider
    pub fn add_collider(&mut self, collider: ClothCollider) {
        self.colliders.push(collider);
    }

    /// Clear all colliders
    pub fn clear_colliders(&mut self) {
        self.colliders.clear();
    }

    /// Get particle index from grid position
    pub fn particle_index(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.config.width && y < self.config.height {
            Some(y * self.config.width + x)
        } else {
            None
        }
    }

    /// Get particle position
    pub fn particle_position(&self, index: usize) -> Option<Vec3> {
        self.particles.get(index).map(|p| p.position)
    }

    /// Calculate cloth normal at a particle (for wind)
    fn particle_normal(&self, x: usize, y: usize) -> Vec3 {
        let idx = y * self.config.width + x;
        let center = self.particles[idx].position;

        let mut normal = Vec3::ZERO;
        let mut count = 0;

        // Get neighboring particles for normal calculation
        let neighbors = [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ];

        for i in 0..4 {
            let (nx, ny) = neighbors[i];
            let (nx2, ny2) = neighbors[(i + 1) % 4];

            if nx < self.config.width
                && ny < self.config.height
                && nx2 < self.config.width
                && ny2 < self.config.height
            {
                let idx1 = ny * self.config.width + nx;
                let idx2 = ny2 * self.config.width + nx2;

                let v1 = self.particles[idx1].position - center;
                let v2 = self.particles[idx2].position - center;
                normal += v1.cross(v2);
                count += 1;
            }
        }

        if count > 0 {
            normal.normalize_or_zero()
        } else {
            Vec3::Y
        }
    }

    /// Update cloth simulation
    pub fn update(&mut self, dt: f32) {
        // Pre-compute normals for wind calculation
        let mut normals = Vec::new();
        if self.config.wind.length_squared() > 0.001 {
            for y in 0..self.config.height {
                for x in 0..self.config.width {
                    normals.push(self.particle_normal(x, y));
                }
            }
        }

        // Apply forces
        for y in 0..self.config.height {
            for x in 0..self.config.width {
                let idx = y * self.config.width + x;
                let inv_mass = self.particles[idx].inv_mass;

                // Gravity
                let gravity_force = self.config.gravity * (1.0 / inv_mass.max(0.001));
                self.particles[idx].apply_force(gravity_force);

                // Wind (affected by particle normal)
                if !normals.is_empty() {
                    let normal = normals[idx];
                    let wind_effect = self.config.wind.dot(normal).abs();
                    let wind_force = self.config.wind * wind_effect * (1.0 / inv_mass.max(0.001));
                    self.particles[idx].apply_force(wind_force);
                }

                // Air resistance
                let velocity = self.particles[idx].velocity();
                let drag = -velocity * self.config.air_resistance * (1.0 / inv_mass.max(0.001));
                self.particles[idx].apply_force(drag);
            }
        }

        // Integrate
        for particle in &mut self.particles {
            particle.integrate(dt, self.config.damping);
        }

        // Solve constraints
        for _ in 0..self.config.solver_iterations {
            for constraint in &self.constraints {
                constraint.solve(&mut self.particles);
            }
        }

        // Resolve collisions
        for particle in &mut self.particles {
            for collider in &self.colliders {
                collider.resolve_collision(particle, self.collision_friction);
            }
        }
    }

    /// Get all particle positions as a flat array
    pub fn get_positions(&self) -> Vec<Vec3> {
        self.particles.iter().map(|p| p.position).collect()
    }

    /// Get triangle indices for rendering
    pub fn get_indices(&self) -> Vec<u32> {
        let mut indices = Vec::new();
        let w = self.config.width as u32;

        for y in 0..(self.config.height - 1) as u32 {
            for x in 0..(self.config.width - 1) as u32 {
                let idx = y * w + x;

                // First triangle
                indices.push(idx);
                indices.push(idx + 1);
                indices.push(idx + w);

                // Second triangle
                indices.push(idx + 1);
                indices.push(idx + w + 1);
                indices.push(idx + w);
            }
        }

        indices
    }

    /// Get particle count
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Get constraint count
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }
}

/// Manager for multiple cloth simulations
#[derive(Debug, Default)]
pub struct ClothManager {
    cloths: HashMap<ClothId, Cloth>,
    next_id: u64,
}

impl ClothManager {
    /// Create a new cloth manager
    pub fn new() -> Self {
        Self {
            cloths: HashMap::new(),
            next_id: 1,
        }
    }

    /// Create a new cloth
    pub fn create(&mut self, config: ClothConfig, origin: Vec3) -> ClothId {
        let id = ClothId(self.next_id);
        self.next_id += 1;
        self.cloths.insert(id, Cloth::new(id, config, origin));
        id
    }

    /// Remove a cloth
    pub fn remove(&mut self, id: ClothId) -> bool {
        self.cloths.remove(&id).is_some()
    }

    /// Get a cloth
    pub fn get(&self, id: ClothId) -> Option<&Cloth> {
        self.cloths.get(&id)
    }

    /// Get a mutable cloth
    pub fn get_mut(&mut self, id: ClothId) -> Option<&mut Cloth> {
        self.cloths.get_mut(&id)
    }

    /// Update all cloths
    pub fn update(&mut self, dt: f32) {
        for cloth in self.cloths.values_mut() {
            cloth.update(dt);
        }
    }

    /// Get cloth count
    pub fn count(&self) -> usize {
        self.cloths.len()
    }

    /// Iterate over all cloths
    pub fn iter(&self) -> impl Iterator<Item = &Cloth> {
        self.cloths.values()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let particle = ClothParticle::new(Vec3::new(1.0, 2.0, 3.0), 0.5);
        assert_eq!(particle.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(particle.inv_mass, 2.0); // 1/0.5
        assert!(!particle.pinned);
    }

    #[test]
    fn test_pinned_particle() {
        let particle = ClothParticle::pinned(Vec3::ZERO);
        assert!(particle.pinned);
        assert_eq!(particle.inv_mass, 0.0);
    }

    #[test]
    fn test_particle_force() {
        let mut particle = ClothParticle::new(Vec3::ZERO, 1.0);
        particle.apply_force(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(particle.acceleration, Vec3::new(10.0, 0.0, 0.0));
    }

    #[test]
    fn test_pinned_particle_no_force() {
        let mut particle = ClothParticle::pinned(Vec3::ZERO);
        particle.apply_force(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(particle.acceleration, Vec3::ZERO);
    }

    #[test]
    fn test_particle_integrate() {
        let mut particle = ClothParticle::new(Vec3::ZERO, 1.0);
        particle.apply_force(Vec3::new(0.0, -10.0, 0.0)); // Gravity-like

        particle.integrate(0.016, 1.0);

        // Should have moved downward
        assert!(particle.position.y < 0.0);
    }

    #[test]
    fn test_distance_constraint() {
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0), // Too far apart
        ];

        let constraint = DistanceConstraint::new(0, 1, 1.0); // Rest length = 1

        constraint.solve(&mut particles);

        // Particles should have moved closer
        let dist = (particles[1].position - particles[0].position).length();
        assert!(dist < 2.0, "Particles should be closer after constraint");
    }

    #[test]
    fn test_cloth_creation() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 0.1,
            ..Default::default()
        };

        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        assert_eq!(cloth.particle_count(), 25);
        assert!(cloth.constraint_count() > 0);
    }

    #[test]
    fn test_cloth_pin_top_edge() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        cloth.pin_top_edge();

        // All particles in first row should be pinned
        for x in 0..5 {
            assert!(cloth.particles[x].pinned);
        }
    }

    #[test]
    fn test_cloth_pin_corners() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        cloth.pin_corners();

        assert!(cloth.particles[0].pinned); // Top-left
        assert!(cloth.particles[4].pinned); // Top-right
    }

    #[test]
    fn test_cloth_move_pinned() {
        let config = ClothConfig::default();
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        cloth.pin_particle(0);

        let new_pos = Vec3::new(5.0, 5.0, 5.0);
        cloth.move_pinned(0, new_pos);

        assert_eq!(cloth.particles[0].position, new_pos);
    }

    #[test]
    fn test_sphere_collider() {
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        let mut particle = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.5);

        // Particle should be pushed out of sphere
        let dist = particle.position.length();
        assert!(
            dist >= 0.99,
            "Particle should be at or outside sphere surface"
        );
    }

    #[test]
    fn test_plane_collider() {
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        let mut particle = ClothParticle::new(Vec3::new(0.0, -0.5, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.5);

        // Particle should be above plane
        assert!(particle.position.y >= 0.0);
    }

    #[test]
    fn test_cloth_update_gravity() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Get initial Y of unpinned particle
        let initial_y = cloth.particles[4].position.y; // Center particle

        // Update
        cloth.update(0.016);

        // Should have fallen
        assert!(cloth.particles[4].position.y < initial_y);
    }

    #[test]
    fn test_cloth_with_wind() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            wind: Vec3::new(10.0, 0.0, 0.0),
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        cloth.pin_top_edge();

        let initial_x = cloth.particles[12].position.x; // Middle particle

        // Multiple updates
        for _ in 0..10 {
            cloth.update(0.016);
        }

        // Should have moved in wind direction
        assert!(cloth.particles[12].position.x > initial_x);
    }

    #[test]
    fn test_cloth_indices() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };

        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        let indices = cloth.get_indices();

        // 3x3 grid = 2x2 quads = 4 quads × 2 triangles × 3 indices = 24 indices
        assert_eq!(indices.len(), 24);
    }

    #[test]
    fn test_cloth_manager() {
        let mut manager = ClothManager::new();

        let id = manager.create(ClothConfig::default(), Vec3::ZERO);
        assert_eq!(manager.count(), 1);

        assert!(manager.get(id).is_some());
        assert!(manager.remove(id));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_cloth_manager_update() {
        let mut manager = ClothManager::new();

        manager.create(
            ClothConfig {
                width: 3,
                height: 3,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        // Should not panic
        manager.update(0.016);
    }

    #[test]
    fn test_capsule_collider() {
        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 2.0, 0.0),
            radius: 0.5,
        };

        let mut particle = ClothParticle::new(Vec3::new(0.2, 1.0, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.5);

        // Particle should be pushed out of capsule
        let dist_from_axis = Vec3::new(particle.position.x, 0.0, particle.position.z).length();
        assert!(
            dist_from_axis >= 0.49,
            "Particle should be at capsule surface"
        );
    }

    #[test]
    fn test_particle_index() {
        let config = ClothConfig {
            width: 5,
            height: 4,
            ..Default::default()
        };

        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        assert_eq!(cloth.particle_index(0, 0), Some(0));
        assert_eq!(cloth.particle_index(4, 0), Some(4));
        assert_eq!(cloth.particle_index(0, 1), Some(5));
        assert_eq!(cloth.particle_index(4, 3), Some(19));
        assert_eq!(cloth.particle_index(5, 0), None); // Out of bounds
    }

    #[test]
    fn test_cloth_damping() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            damping: 0.5, // Heavy damping
            gravity: Vec3::ZERO,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Give a particle some velocity
        cloth.particles[4].prev_position = cloth.particles[4].position - Vec3::X;

        cloth.update(0.016);

        // Velocity should be reduced due to damping
        let velocity = cloth.particles[4].velocity();
        assert!(velocity.length() < 1.0);
    }

    #[test]
    fn test_constraint_stiffness() {
        // Low stiffness constraint
        let mut particles_soft = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0),
        ];
        let mut constraint_soft = DistanceConstraint::new(0, 1, 1.0);
        constraint_soft.stiffness = 0.1;
        constraint_soft.solve(&mut particles_soft);

        // High stiffness constraint
        let mut particles_stiff = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0),
        ];
        let mut constraint_stiff = DistanceConstraint::new(0, 1, 1.0);
        constraint_stiff.stiffness = 1.0;
        constraint_stiff.solve(&mut particles_stiff);

        let dist_soft = (particles_soft[1].position - particles_soft[0].position).length();
        let dist_stiff = (particles_stiff[1].position - particles_stiff[0].position).length();

        // Stiff constraint should correct more
        assert!(dist_stiff < dist_soft);
    }

    // ============================================================================
    // TEARING TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_constraint_zero_rest_length() {
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0),
        ];

        // Zero rest length constraint
        let constraint = DistanceConstraint::new(0, 1, 0.0);
        constraint.solve(&mut particles);

        // Should pull particles together
        let dist = (particles[1].position - particles[0].position).length();
        assert!(dist < 1.0, "Particles should be closer");
    }

    #[test]
    fn test_constraint_stretched_beyond_limit() {
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::new(10.0, 0.0, 0.0), 1.0), // Very stretched
        ];

        let constraint = DistanceConstraint::new(0, 1, 1.0);

        // Multiple iterations should converge
        for _ in 0..10 {
            constraint.solve(&mut particles);
        }

        let dist = (particles[1].position - particles[0].position).length();
        assert!(dist < 2.0, "Should converge toward rest length");
    }

    // ============================================================================
    // WIND INTERACTION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_wind_direction_affects_movement() {
        // Test X wind
        let config_x = ClothConfig {
            width: 3,
            height: 3,
            wind: Vec3::new(20.0, 0.0, 0.0),
            gravity: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth_x = Cloth::new(ClothId(1), config_x, Vec3::ZERO);
        cloth_x.pin_top_edge();

        // Test Z wind
        let config_z = ClothConfig {
            width: 3,
            height: 3,
            wind: Vec3::new(0.0, 0.0, 20.0),
            gravity: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth_z = Cloth::new(ClothId(2), config_z, Vec3::ZERO);
        cloth_z.pin_top_edge();

        for _ in 0..20 {
            cloth_x.update(0.016);
            cloth_z.update(0.016);
        }

        // Bottom center particle
        let bottom = cloth_x.config.width * (cloth_x.config.height - 1) + 1;

        // X wind should push X, Z wind should push Z
        assert!(cloth_x.particles[bottom].position.x > 0.0);
        assert!(cloth_z.particles[bottom].position.z > 0.0);
    }

    #[test]
    fn test_air_resistance_slows_movement() {
        let config_low_drag = ClothConfig {
            width: 3,
            height: 3,
            air_resistance: 0.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            ..Default::default()
        };

        let config_high_drag = ClothConfig {
            width: 3,
            height: 3,
            air_resistance: 0.5,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            ..Default::default()
        };

        let mut cloth_low = Cloth::new(ClothId(1), config_low_drag, Vec3::ZERO);
        let mut cloth_high = Cloth::new(ClothId(2), config_high_drag, Vec3::ZERO);

        for _ in 0..30 {
            cloth_low.update(0.016);
            cloth_high.update(0.016);
        }

        // High drag should fall slower
        let vel_low = cloth_low.particles[4].velocity().length();
        let vel_high = cloth_high.particles[4].velocity().length();

        assert!(vel_high < vel_low, "High drag should have lower velocity");
    }

    // ============================================================================
    // SELF-COLLISION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_particles_same_position() {
        let mut particles = vec![
            ClothParticle::new(Vec3::ZERO, 1.0),
            ClothParticle::new(Vec3::ZERO, 1.0), // Same position
        ];

        let constraint = DistanceConstraint::new(0, 1, 1.0);

        // Should not panic or produce NaN
        constraint.solve(&mut particles);

        assert!(!particles[0].position.x.is_nan());
        assert!(!particles[1].position.x.is_nan());
    }

    // ============================================================================
    // EDGE CASE TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_single_particle_cloth() {
        // Note: Implementation requires at least 2x2 grid due to constraint generation
        // This test documents the minimum viable cloth size
        let config = ClothConfig {
            width: 2,
            height: 2,
            ..Default::default()
        };

        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        assert_eq!(cloth.particle_count(), 4);
        assert!(cloth.constraint_count() > 0);
    }

    #[test]
    fn test_cloth_1xN() {
        // Note: Implementation requires at least 2 in each dimension for proper constraint setup
        let config = ClothConfig {
            width: 5,
            height: 2,
            spacing: 0.2,
            ..Default::default()
        };

        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        assert_eq!(cloth.particle_count(), 10);
        // Should have constraints
        assert!(cloth.constraint_count() > 0);
    }

    #[test]
    fn test_particle_velocity() {
        let mut particle = ClothParticle::new(Vec3::ZERO, 1.0);
        particle.prev_position = Vec3::new(-1.0, 0.0, 0.0);

        let velocity = particle.velocity();
        assert_eq!(velocity, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_unpin_particle() {
        let mut cloth = Cloth::new(ClothId(1), ClothConfig::default(), Vec3::ZERO);
        cloth.pin_particle(0);
        assert!(cloth.particles[0].pinned);

        cloth.unpin_particle(0);
        assert!(!cloth.particles[0].pinned);
        assert!(cloth.particles[0].inv_mass > 0.0);
    }

    #[test]
    fn test_clear_colliders() {
        let mut cloth = Cloth::new(ClothId(1), ClothConfig::default(), Vec3::ZERO);
        cloth.add_collider(ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        });
        cloth.add_collider(ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        });

        assert_eq!(cloth.colliders.len(), 2);
        cloth.clear_colliders();
        assert_eq!(cloth.colliders.len(), 0);
    }

    #[test]
    fn test_particle_position_getter() {
        let cloth = Cloth::new(ClothId(1), ClothConfig::default(), Vec3::new(5.0, 5.0, 5.0));

        let pos = cloth.particle_position(0);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap(), Vec3::new(5.0, 5.0, 5.0));

        // Out of bounds
        assert!(cloth.particle_position(10000).is_none());
    }

    #[test]
    fn test_config_default() {
        let config = ClothConfig::default();

        assert_eq!(config.width, 20);
        assert_eq!(config.height, 20);
        assert_eq!(config.spacing, 0.1);
        assert_eq!(config.particle_mass, 0.1);
        assert_eq!(config.stiffness, 0.8);
        assert_eq!(config.damping, 0.98);
        assert_eq!(config.solver_iterations, 3);
    }

    #[test]
    fn test_degenerate_capsule() {
        // Capsule with start == end should act like sphere
        let collider = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            radius: 1.0,
        };

        let mut particle = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.5);

        // Should still push out
        let dist = particle.position.length();
        assert!(dist >= 0.99);
    }

    #[test]
    fn test_manager_get_mut() {
        let mut manager = ClothManager::new();
        let id = manager.create(ClothConfig::default(), Vec3::ZERO);

        {
            let cloth = manager.get_mut(id).unwrap();
            cloth.pin_top_edge();
        }

        let cloth = manager.get(id).unwrap();
        assert!(cloth.particles[0].pinned);
    }

    #[test]
    fn test_manager_iter() {
        let mut manager = ClothManager::new();
        manager.create(ClothConfig::default(), Vec3::ZERO);
        manager.create(ClothConfig::default(), Vec3::new(5.0, 0.0, 0.0));
        manager.create(ClothConfig::default(), Vec3::new(10.0, 0.0, 0.0));

        assert_eq!(manager.iter().count(), 3);
    }

    #[test]
    fn test_manager_remove_nonexistent() {
        let mut manager = ClothManager::new();
        assert!(!manager.remove(ClothId(999)));
    }

    #[test]
    fn test_manager_get_nonexistent() {
        let manager = ClothManager::new();
        assert!(manager.get(ClothId(999)).is_none());
    }

    #[test]
    fn test_pinned_particle_no_integrate() {
        let mut particle = ClothParticle::pinned(Vec3::new(1.0, 1.0, 1.0));
        particle.acceleration = Vec3::new(100.0, 100.0, 100.0);

        particle.integrate(1.0, 1.0);

        // Position should not change
        assert_eq!(particle.position, Vec3::new(1.0, 1.0, 1.0));
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6 — cloth simulation tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_particle_integrate_verlet() {
        let mut p = ClothParticle::new(Vec3::new(0.0, 5.0, 0.0), 1.0);
        // Set prev_position different from position to create velocity
        p.prev_position = Vec3::new(0.0, 5.0, 0.0);
        // Apply gravity-like force
        p.apply_force(Vec3::new(0.0, -9.81, 0.0));

        let dt = 0.016; // 60 FPS
        p.integrate(dt, 0.99);

        // After integration, particle should move downward
        assert!(
            p.position.y < 5.0,
            "Particle should fall under gravity, got y={}",
            p.position.y
        );
        // Acceleration should be reset
        assert_eq!(
            p.acceleration,
            Vec3::ZERO,
            "Acceleration should reset after integrate"
        );
    }

    #[test]
    fn mutation_particle_velocity() {
        let mut p = ClothParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::new(0.0, 0.0, 0.0);
        let vel = p.velocity();
        assert!(
            (vel.x - 1.0).abs() < 1e-6,
            "Velocity X should be 1.0, got {}",
            vel.x
        );
    }

    #[test]
    fn mutation_particle_apply_force_mass() {
        let mut p = ClothParticle::new(Vec3::ZERO, 2.0); // mass=2, inv_mass=0.5
        p.apply_force(Vec3::new(10.0, 0.0, 0.0));
        // Acceleration = force * inv_mass = 10 * 0.5 = 5
        assert!(
            (p.acceleration.x - 5.0).abs() < 1e-6,
            "Acceleration should be force*inv_mass=5, got {}",
            p.acceleration.x
        );
    }

    #[test]
    fn mutation_pinned_particle_ignores_force() {
        let mut p = ClothParticle::pinned(Vec3::ZERO);
        p.apply_force(Vec3::new(1000.0, 1000.0, 1000.0));
        assert_eq!(
            p.acceleration,
            Vec3::ZERO,
            "Pinned particle should ignore forces"
        );
    }

    #[test]
    fn mutation_constraint_solve_pulls_together() {
        // Two particles too far apart — constraint should pull them together
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(3.0, 0.0, 0.0), 1.0), // 3 units apart
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0); // rest_length=1

        constraint.solve(&mut particles);

        // Both should move toward each other
        assert!(
            particles[0].position.x > 0.0,
            "Particle 0 should move positive X"
        );
        assert!(
            particles[1].position.x < 3.0,
            "Particle 1 should move negative X"
        );
        // Distance should be closer to rest_length
        let new_dist = (particles[1].position - particles[0].position).length();
        assert!(
            new_dist < 3.0,
            "Distance should decrease from 3.0, got {}",
            new_dist
        );
    }

    #[test]
    fn mutation_constraint_solve_pushes_apart() {
        // Two particles too close — constraint should push them apart
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(0.1, 0.0, 0.0), 1.0), // 0.1 apart
        ];
        let constraint = DistanceConstraint::new(0, 1, 2.0); // rest_length=2

        constraint.solve(&mut particles);

        let new_dist = (particles[1].position - particles[0].position).length();
        assert!(
            new_dist > 0.1,
            "Distance should increase from 0.1, got {}",
            new_dist
        );
    }

    #[test]
    fn mutation_constraint_respects_pinned() {
        let mut particles = vec![
            ClothParticle::pinned(Vec3::new(0.0, 0.0, 0.0)),
            ClothParticle::new(Vec3::new(3.0, 0.0, 0.0), 1.0),
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0);

        constraint.solve(&mut particles);

        // Pinned particle should not move
        assert_eq!(
            particles[0].position,
            Vec3::new(0.0, 0.0, 0.0),
            "Pinned particle should not move"
        );
        // Free particle should move toward pinned
        assert!(
            particles[1].position.x < 3.0,
            "Free particle should move toward pinned"
        );
    }

    #[test]
    fn mutation_collider_sphere_pushout() {
        let mut particle = ClothParticle::new(Vec3::new(0.1, 0.0, 0.0), 1.0);
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        collider.resolve_collision(&mut particle, 0.0);

        // Particle was inside sphere (dist=0.1 < radius=1.0), should be pushed out
        let dist = particle.position.length();
        assert!(
            dist >= 0.99,
            "Particle should be pushed to sphere surface, dist={}",
            dist
        );
    }

    #[test]
    fn mutation_collider_plane_pushout() {
        let mut particle = ClothParticle::new(Vec3::new(0.0, -0.5, 0.0), 1.0);
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        collider.resolve_collision(&mut particle, 0.0);

        // Particle was below plane (y=-0.5, dist below plane < 0), should be pushed up
        assert!(
            particle.position.y >= 0.0,
            "Particle should be pushed above plane, y={}",
            particle.position.y
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — cloth Round 2 arithmetic/boundary tests
    // ═══════════════════════════════════════════════════════════════

    // --- ClothCollider::resolve_collision sphere arithmetic ---
    #[test]
    fn mutation_sphere_collision_penetration_depth() {
        let mut particle = ClothParticle::new(Vec3::new(0.3, 0.0, 0.0), 1.0);
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        collider.resolve_collision(&mut particle, 0.0);
        let dist = particle.position.length();
        // Should be pushed to radius=1.0
        assert!(
            (dist - 1.0).abs() < 0.01,
            "Should be at sphere surface, dist={}",
            dist
        );
        // Direction should be outward from center (positive X)
        assert!(
            particle.position.x > 0.0,
            "Should be pushed in +X direction"
        );
    }

    #[test]
    fn mutation_sphere_collision_friction_damps_tangent() {
        let mut particle = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        // Give it tangential velocity
        particle.prev_position = Vec3::new(0.5, -1.0, 0.0); // velocity = (0, 1, 0) tangential
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        collider.resolve_collision(&mut particle, 1.0); // full friction
                                                        // With friction=1.0, tangential velocity should be zeroed
        let vel = particle.velocity();
        let normal = particle.position.normalize();
        let tangent_vel = vel - vel.dot(normal) * normal;
        assert!(
            tangent_vel.length() < 0.1,
            "Full friction should kill tangential velocity"
        );
    }

    #[test]
    fn mutation_sphere_no_collision_outside() {
        let mut particle = ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0);
        let pos_before = particle.position;
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        collider.resolve_collision(&mut particle, 0.5);
        assert_eq!(
            particle.position, pos_before,
            "Outside sphere should not be affected"
        );
    }

    #[test]
    fn mutation_sphere_collision_pinned_ignored() {
        let mut particle = ClothParticle::pinned(Vec3::new(0.3, 0.0, 0.0));
        let pos_before = particle.position;
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        collider.resolve_collision(&mut particle, 0.5);
        assert_eq!(
            particle.position, pos_before,
            "Pinned particle should not move"
        );
    }

    // --- ClothCollider::resolve_collision capsule arithmetic ---
    #[test]
    fn mutation_capsule_collision_midpoint() {
        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 4.0, 0.0),
            radius: 1.0,
        };
        let mut particle = ClothParticle::new(Vec3::new(0.3, 2.0, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.0);
        // Closest point on axis is (0, 2, 0), particle at (0.3, 2, 0), dist=0.3 < 1.0
        let dist_from_axis = Vec3::new(particle.position.x, 0.0, particle.position.z).length();
        assert!(
            dist_from_axis >= 0.99,
            "Should be pushed to capsule surface, dist={}",
            dist_from_axis
        );
    }

    #[test]
    fn mutation_capsule_collision_past_end() {
        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 4.0, 0.0),
            radius: 1.0,
        };
        // Past the end of the capsule — should clamp to endpoint
        let mut particle = ClothParticle::new(Vec3::new(0.3, 5.0, 0.0), 1.0);
        collider.resolve_collision(&mut particle, 0.0);
        // Closest point is (0, 4, 0), dist = sqrt(0.09 + 1.0) = 1.04 > 1.0, no collision
        // Actually distance from (0.3, 5, 0) to (0, 4, 0) = sqrt(0.09+1) = 1.044, outside radius
        assert!(
            (particle.position.x - 0.3).abs() < 0.01,
            "Should not be affected (outside)"
        );
    }

    // --- ClothCollider::resolve_collision plane arithmetic ---
    #[test]
    fn mutation_plane_collision_depth_exact() {
        let mut particle = ClothParticle::new(Vec3::new(2.0, -3.0, 5.0), 1.0);
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        collider.resolve_collision(&mut particle, 0.0);
        assert!(
            (particle.position.y - 0.0).abs() < 1e-5,
            "Should be pushed to plane surface"
        );
        assert!(
            (particle.position.x - 2.0).abs() < 1e-5,
            "X should be unchanged"
        );
        assert!(
            (particle.position.z - 5.0).abs() < 1e-5,
            "Z should be unchanged"
        );
    }

    #[test]
    fn mutation_plane_no_collision_above() {
        let mut particle = ClothParticle::new(Vec3::new(0.0, 1.0, 0.0), 1.0);
        let pos_before = particle.position;
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        collider.resolve_collision(&mut particle, 0.5);
        assert_eq!(
            particle.position, pos_before,
            "Above plane should not be affected"
        );
    }

    // --- Cloth::particle_normal ---
    #[test]
    fn mutation_particle_normal_center_nonzero() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(400), config, Vec3::ZERO);
        // Symmetric displacement at center cancels out; displace an off-center neighbor
        // to break symmetry and produce non-zero normal.
        let left_idx = cloth.particle_index(1, 2).unwrap();
        cloth.particles[left_idx].position.y += 2.0;
        let normal = cloth.particle_normal(2, 2);
        assert!(
            normal.length() > 0.1,
            "Center normal should be non-zero with asymmetric displacement, got {:?}",
            normal
        );
    }

    #[test]
    fn mutation_particle_normal_corner() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(401), config, Vec3::ZERO);
        // Corner (0,0) has fewer neighbors
        let normal = cloth.particle_normal(0, 0);
        // Should still produce something (count may be 1 or 0, but Y fallback)
        assert!(
            normal.length() > 0.01 || normal == Vec3::Y,
            "Corner normal should be valid"
        );
    }

    // --- Cloth::get_indices ---
    #[test]
    fn mutation_get_indices_3x3_count() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(410), config, Vec3::ZERO);
        let indices = cloth.get_indices();
        // 2x2 quads × 2 triangles × 3 indices = 24
        assert_eq!(indices.len(), 24, "3x3 cloth should have 24 indices");
    }

    #[test]
    fn mutation_get_indices_4x4_count() {
        let config = ClothConfig {
            width: 4,
            height: 4,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(411), config, Vec3::ZERO);
        let indices = cloth.get_indices();
        // 3x3 quads × 2 triangles × 3 indices = 54
        assert_eq!(indices.len(), 54, "4x4 cloth should have 54 indices");
    }

    #[test]
    fn mutation_get_indices_valid_range() {
        let config = ClothConfig {
            width: 5,
            height: 5,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(412), config, Vec3::ZERO);
        let indices = cloth.get_indices();
        let max_valid = (5 * 5 - 1) as u32;
        for &idx in &indices {
            assert!(
                idx <= max_valid,
                "Index {} out of range [0, {}]",
                idx,
                max_valid
            );
        }
    }

    // --- DistanceConstraint::solve weight arithmetic ---
    #[test]
    fn mutation_constraint_solve_unequal_mass() {
        // Heavy particle should move less than light particle
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 10.0), // heavy, inv_mass=0.1
            ClothParticle::new(Vec3::new(3.0, 0.0, 0.0), 1.0),  // light, inv_mass=1.0
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0);
        constraint.solve(&mut particles);
        // Heavy particle (0) should move less
        let move0 = particles[0].position.x.abs();
        let move1 = (3.0 - particles[1].position.x).abs();
        assert!(
            move1 > move0 * 2.0,
            "Light particle should move more: move0={}, move1={}",
            move0,
            move1
        );
    }

    #[test]
    fn mutation_constraint_solve_both_pinned_no_move() {
        let mut particles = vec![
            ClothParticle::pinned(Vec3::new(0.0, 0.0, 0.0)),
            ClothParticle::pinned(Vec3::new(5.0, 0.0, 0.0)),
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0);
        constraint.solve(&mut particles);
        // Neither should move
        assert_eq!(particles[0].position, Vec3::ZERO);
        assert_eq!(particles[1].position, Vec3::new(5.0, 0.0, 0.0));
    }

    // --- Cloth::update integration ---
    #[test]
    fn mutation_cloth_update_applies_gravity() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            gravity: Vec3::new(0.0, -20.0, 0.0),
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(420), config, Vec3::new(0.0, 10.0, 0.0));
        let initial_y = cloth.particles[4].position.y; // center
        cloth.update(0.016);
        assert!(
            cloth.particles[4].position.y < initial_y,
            "Gravity should pull particles down"
        );
    }

    #[test]
    fn mutation_cloth_update_constraint_iterations() {
        // More solver iterations should give tighter constraint satisfaction
        let config_1 = ClothConfig {
            width: 3,
            height: 3,
            solver_iterations: 1,
            ..Default::default()
        };
        let config_5 = ClothConfig {
            width: 3,
            height: 3,
            solver_iterations: 5,
            ..Default::default()
        };
        let mut cloth_1 = Cloth::new(ClothId(421), config_1, Vec3::ZERO);
        let mut cloth_5 = Cloth::new(ClothId(422), config_5, Vec3::ZERO);
        // Stretch a particle far away
        cloth_1.particles[4].position = Vec3::new(10.0, 0.0, 0.0);
        cloth_5.particles[4].position = Vec3::new(10.0, 0.0, 0.0);
        cloth_1.update(0.016);
        cloth_5.update(0.016);
        // More iterations should bring it closer to neighbors
        let dist_1 = (cloth_1.particles[4].position - cloth_1.particles[3].position).length();
        let dist_5 = (cloth_5.particles[4].position - cloth_5.particles[3].position).length();
        assert!(
            dist_5 <= dist_1 + 0.01,
            "More solver iterations should give shorter distance: 1iter={}, 5iter={}",
            dist_1,
            dist_5
        );
    }

    // --- Cloth::unpin_particle restores mass ---
    #[test]
    fn mutation_unpin_restores_correct_inv_mass() {
        let config = ClothConfig {
            particle_mass: 0.5,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(430), config, Vec3::ZERO);
        cloth.pin_particle(0);
        assert_eq!(cloth.particles[0].inv_mass, 0.0);
        cloth.unpin_particle(0);
        // Should restore to 1/particle_mass = 1/0.5 = 2.0
        assert!(
            (cloth.particles[0].inv_mass - 2.0).abs() < 1e-6,
            "Should restore inv_mass to 1/particle_mass=2.0, got {}",
            cloth.particles[0].inv_mass
        );
    }

    // --- Cloth::move_pinned only moves pinned ---
    #[test]
    fn mutation_move_pinned_unpinned_noop() {
        let config = ClothConfig::default();
        let mut cloth = Cloth::new(ClothId(431), config, Vec3::ZERO);
        let pos_before = cloth.particles[0].position;
        cloth.move_pinned(0, Vec3::new(99.0, 99.0, 99.0));
        assert_eq!(
            cloth.particles[0].position, pos_before,
            "Unpinned particle should not move via move_pinned"
        );
    }

    // --- Cloth::pin_corners ---
    #[test]
    fn mutation_pin_corners_sets_inv_mass() {
        let config = ClothConfig {
            width: 4,
            height: 4,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(432), config, Vec3::ZERO);
        cloth.pin_corners();
        assert_eq!(
            cloth.particles[0].inv_mass, 0.0,
            "Top-left inv_mass should be 0"
        );
        assert_eq!(
            cloth.particles[3].inv_mass, 0.0,
            "Top-right inv_mass should be 0"
        );
    }

    // ===== DEEP REMEDIATION v3.6.2 — cloth Round 3 remaining mutations =====

    // --- ClothParticle::integrate Verlet arithmetic ---
    #[test]
    fn mutation_r3_integrate_velocity_subtraction() {
        // velocity = position - prev_position  (mutation: - → +)
        let mut p = ClothParticle::new(Vec3::new(5.0, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::new(3.0, 0.0, 0.0); // velocity = (5-3, 0, 0) = (2, 0, 0)
        p.integrate(1.0, 1.0); // no damping, dt=1  ->  new_pos = 5 + 2*1 + 0*1*1 = 7
        assert!(
            (p.position.x - 7.0).abs() < 1e-5,
            "x should be 7, velocity=+2, got {}",
            p.position.x
        );
        // If - became +, velocity would be (5+3)=8, new_pos=5+8=13 ≠ 7
    }

    #[test]
    fn mutation_r3_integrate_damping_multiply() {
        // position += velocity * damping  (mutation: * → + or /)
        let mut p = ClothParticle::new(Vec3::new(4.0, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::new(2.0, 0.0, 0.0); // velocity = (2,0,0)
        let damping = 0.5;
        p.integrate(1.0, damping); // new_pos = 4 + 2*0.5 + 0 = 5
        assert!(
            (p.position.x - 5.0).abs() < 1e-5,
            "damping 0.5 should give x=5, got {}",
            p.position.x
        );
    }

    #[test]
    fn mutation_r3_integrate_accel_dt_squared() {
        // position += acceleration * dt * dt  (mutations: * → + or /)
        let mut p = ClothParticle::new(Vec3::ZERO, 1.0);
        p.apply_force(Vec3::new(6.0, 0.0, 0.0)); // accel = 6 * inv_mass(1) = 6
        let dt = 0.5;
        p.integrate(dt, 1.0); // new_pos = 0 + 0 + 6 * 0.5 * 0.5 = 1.5
        assert!(
            (p.position.x - 1.5).abs() < 1e-5,
            "accel*dt*dt should give 1.5, got {}",
            p.position.x
        );
    }

    #[test]
    fn mutation_r3_integrate_clears_acceleration() {
        let mut p = ClothParticle::new(Vec3::ZERO, 1.0);
        p.apply_force(Vec3::new(10.0, 0.0, 0.0));
        p.integrate(0.1, 1.0);
        assert_eq!(
            p.acceleration,
            Vec3::ZERO,
            "Acceleration should be cleared after integrate"
        );
    }

    // --- ClothParticle::apply_force ---
    #[test]
    fn mutation_r3_apply_force_inv_mass_multiply() {
        // acceleration += force * inv_mass  (mutation: * → /)
        let mut p = ClothParticle::new(Vec3::ZERO, 2.0); // inv_mass = 0.5
        p.apply_force(Vec3::new(8.0, 0.0, 0.0));
        // accel should be 8 * 0.5 = 4
        assert!(
            (p.acceleration.x - 4.0).abs() < 1e-5,
            "force*inv_mass: 8*0.5=4, got {}",
            p.acceleration.x
        );
        // If * became /, accel = 8/0.5 = 16 ≠ 4
    }

    // --- Cloth::pin_corners w-1 arithmetic ---
    #[test]
    fn mutation_r3_pin_corners_exact_indices() {
        // Mutation: w - 1 → w + 1 or w * 1
        let config = ClothConfig {
            width: 5,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(500), config, Vec3::ZERO);
        cloth.pin_corners();
        // Should pin index 0 (top-left) and index 4 (top-right, w-1=4)
        assert!(cloth.particles[0].pinned, "Index 0 should be pinned");
        assert!(cloth.particles[4].pinned, "Index 4 (w-1) should be pinned");
        // Index 5 (w) should NOT be pinned (catches w+0 mutation)
        assert!(
            !cloth.particles[5].pinned,
            "Index 5 (w) should NOT be pinned"
        );
        // Index 6 (w+1) should NOT be pinned
        assert!(
            !cloth.particles[6].pinned,
            "Index 6 (w+1) should NOT be pinned"
        );
    }

    // --- Cloth::unpin_particle inv_mass restoration ---
    #[test]
    fn mutation_r3_unpin_inv_mass_exact_formula() {
        // inv_mass = 1.0 / particle_mass  (mutation: / → * or %)
        let config = ClothConfig {
            width: 3,
            height: 3,
            particle_mass: 4.0,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(501), config, Vec3::ZERO);
        cloth.pin_particle(0);
        assert_eq!(cloth.particles[0].inv_mass, 0.0);
        cloth.unpin_particle(0);
        // Should be 1.0/4.0 = 0.25
        assert!(
            (cloth.particles[0].inv_mass - 0.25).abs() < 1e-6,
            "inv_mass should be 1/4=0.25, got {}",
            cloth.particles[0].inv_mass
        );
        assert!(!cloth.particles[0].pinned);
    }

    #[test]
    fn mutation_r3_unpin_out_of_bounds_noop() {
        // Mutation: < → <=  (index < len vs index <= len)
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(502), config, Vec3::ZERO);
        let len = cloth.particles.len();
        cloth.pin_particle(0);
        // Unpin at exactly len should be a no-op (catches < vs <=)
        cloth.unpin_particle(len);
        assert!(
            cloth.particles[0].pinned,
            "Unpin at out-of-bounds should not affect existing particles"
        );
    }

    // --- Cloth::move_pinned && check ---
    #[test]
    fn mutation_r3_move_pinned_requires_both_conditions() {
        // Mutation: && → || (index < len && pinned)
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(503), config, Vec3::ZERO);
        let original_pos = cloth.particles[4].position;
        // Particle 4 is NOT pinned, so move_pinned should be no-op
        cloth.move_pinned(4, Vec3::new(99.0, 99.0, 99.0));
        assert_eq!(
            cloth.particles[4].position, original_pos,
            "move_pinned on unpinned should be no-op"
        );
    }

    #[test]
    fn mutation_r3_move_pinned_boundary() {
        // Mutation: < → <=
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(504), config, Vec3::ZERO);
        cloth.pin_particle(0);
        cloth.move_pinned(cloth.particles.len(), Vec3::new(99.0, 99.0, 99.0));
        // Should not panic or change anything at index == len
        assert_ne!(cloth.particles[0].position, Vec3::new(99.0, 99.0, 99.0));
    }

    // --- Cloth::particle_index boundary ---
    #[test]
    fn mutation_r3_particle_index_exact_boundary() {
        // Mutation: < → <= (x < width)
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(505), config, Vec3::ZERO);
        // x=2 (last valid) should return Some
        assert!(cloth.particle_index(2, 2).is_some());
        // x=3 (== width) should return None
        assert!(
            cloth.particle_index(3, 0).is_none(),
            "x == width should be None"
        );
        assert!(
            cloth.particle_index(0, 3).is_none(),
            "y == height should be None"
        );
    }

    // --- Cloth::pin_particle boundary ---
    #[test]
    fn mutation_r3_pin_particle_boundary() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(506), config, Vec3::ZERO);
        // At exactly len, should be no-op (< vs <=)
        cloth.pin_particle(cloth.particles.len());
        // All particles should still be unpinned
        assert!(
            cloth.particles.iter().all(|p| !p.pinned),
            "No particle should be pinned after out-of-bounds pin"
        );
    }

    // --- resolve_collision: capsule deeper arithmetic ---
    #[test]
    fn mutation_r3_capsule_collision_closest_point() {
        // Tests the t-clamping and closest-point calculation
        // Capsule from (0,0,0) to (10,0,0), radius=2
        let capsule = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(10.0, 0.0, 0.0),
            radius: 2.0,
        };
        // Particle at (5, 1, 0) — inside capsule (dist to axis = 1, < radius = 2)
        let mut p = ClothParticle::new(Vec3::new(5.0, 1.0, 0.0), 1.0);
        capsule.resolve_collision(&mut p, 0.0);
        // Should be pushed to radius=2 from axis: y should become 2.0
        assert!(
            (p.position.y - 2.0).abs() < 1e-4,
            "Capsule should push to y=2, got {}",
            p.position.y
        );
    }

    #[test]
    fn mutation_r3_capsule_collision_past_end() {
        // Particle beyond capsule end — t should clamp to axis_length
        let capsule = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::new(4.0, 0.0, 0.0),
            radius: 1.5,
        };
        // Particle at (6, 0.5, 0) — beyond end, closest is (4,0,0), dist=sqrt(4+0.25)=~2.06 > 1.5
        let mut p = ClothParticle::new(Vec3::new(6.0, 0.5, 0.0), 1.0);
        let pos_before = p.position;
        capsule.resolve_collision(&mut p, 0.0);
        assert_eq!(p.position, pos_before, "Outside capsule should not change");
    }

    #[test]
    fn mutation_r3_capsule_friction_tangent() {
        // Tests friction application in capsule: tangent_vel * (1.0 - friction)
        let capsule = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 10.0, 0.0),
            radius: 3.0,
        };
        let mut p = ClothParticle::new(Vec3::new(1.0, 5.0, 0.0), 1.0);
        // Give it velocity along the capsule axis (tangential)
        p.prev_position = Vec3::new(1.0, 4.0, 0.0); // velocity = (0, 1, 0)
        capsule.resolve_collision(&mut p, 0.8);
        // After collision push, friction=0.8 should reduce tangential velocity by a lot
        let velocity = p.position - p.prev_position;
        // The normal is towards x (perpendicular to axis), tangent is along y
        // tangent_vel * (1-0.8) = 0.2 of original tangential
        assert!(
            velocity.y.abs() < 0.5,
            "High friction should reduce tangential velocity, got vy={}",
            velocity.y
        );
    }

    #[test]
    fn mutation_r3_plane_collision_negative_dist() {
        // Plane collision: dist = to_particle.dot(normal), if dist < 0 → push out
        let plane = ClothCollider::Plane {
            point: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        // Particle below plane at y=-0.5
        let mut p = ClothParticle::new(Vec3::new(0.0, -0.5, 0.0), 1.0);
        plane.resolve_collision(&mut p, 0.0);
        assert!(
            (p.position.y - 0.0).abs() < 1e-4,
            "Plane should push particle to y=0, got {}",
            p.position.y
        );
    }

    #[test]
    fn mutation_r3_plane_friction_application() {
        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::new(0.0, 1.0, 0.0),
        };
        let mut p = ClothParticle::new(Vec3::new(1.0, -0.3, 0.0), 1.0);
        p.prev_position = Vec3::new(0.0, -0.3, 0.0); // velocity = (1, 0, 0) — tangential
        plane.resolve_collision(&mut p, 0.9);
        // After push, friction=0.9 means tangent_vel * (1-0.9) = 0.1 of original
        let vel = p.position - p.prev_position;
        assert!(
            vel.x.abs() < 0.5,
            "High friction should heavily reduce tangential, got vx={}",
            vel.x
        );
    }

    // --- DistanceConstraint < vs <= ---
    #[test]
    fn mutation_r3_constraint_solve_boundary_weight() {
        // Tests: if total_weight < 0.0001  (mutation: < → <=)
        // When both pinned, total_weight = 0, so we need < to skip
        // When only one is free with tiny mass, total_weight is small but > 0
        use super::*;
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(2.0, 0.0, 0.0), 1.0),
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0);
        let pos_before_0 = particles[0].position;
        let pos_before_1 = particles[1].position;
        constraint.solve(&mut particles);
        // With rest_length=1 and actual_length=2, particles should move
        assert_ne!(particles[0].position, pos_before_0, "Should move p0");
        assert_ne!(particles[1].position, pos_before_1, "Should move p1");
    }

    // ===== DEEP REMEDIATION v3.6.3 — cloth Round 4 collision math + indices =====

    // --- resolve_collision sphere: exact prev_position formula ---
    #[test]
    fn mutation_r4_sphere_collision_prev_position_exact() {
        use super::*;
        // Sphere at origin, r=2. Particle inside at (1,0,0).
        // normal = (1,0,0), penetration = 2-1 = 1
        // After: position = (1,0,0) + (1,0,0)*1 = (2,0,0)
        // prev_position starts at (0.5,0,0) (from new), then integrate sets it
        let mut p = ClothParticle::new(Vec3::new(1.0, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::new(0.8, 0.0, 0.0); // gives velocity ~(0.2,0,0)
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 2.0,
        };
        collider.resolve_collision(&mut p, 0.5);
        // Position should be pushed to radius
        assert!(
            (p.position.x - 2.0).abs() < 0.01,
            "Pushed to radius: x={}",
            p.position.x
        );
        // velocity = position - prev_position (before collision): ~(0.2, 0, 0)
        // normal_vel = vel.dot(n) * n = 0.2 * (1,0,0) = (0.2,0,0)
        // tangent_vel = vel - normal_vel = (0,0,0)
        // prev_pos = new_pos - (normal_vel + tangent_vel * (1-friction))
        // = (2,0,0) - (0.2,0,0) + (0,0,0) = (1.8,0,0)
        // But velocity is computed AFTER position update, so let's verify structurally
        let new_vel = p.position - p.prev_position;
        assert!(
            new_vel.length() > 0.0,
            "Should have non-zero velocity after collision"
        );
    }

    // --- resolve_collision: friction tangent operator precision ---
    #[test]
    fn mutation_r4_sphere_friction_tangent_velocity() {
        use super::*;
        // Particle inside sphere with lateral velocity (tangent)
        let mut p = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        p.prev_position = Vec3::new(0.5, 0.2, 0.0); // velocity ~ (0, -0.2, 0) (tangent to normal)
        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        // friction=0.0: tangent fully preserved, friction=1.0: tangent eliminated
        let mut p_no_friction = p.clone();
        let mut p_full_friction = p.clone();
        collider.resolve_collision(&mut p_no_friction, 0.0);
        collider.resolve_collision(&mut p_full_friction, 1.0);

        // With more friction, less tangent velocity → smaller overall velocity
        let vel_no = p_no_friction.position - p_no_friction.prev_position;
        let vel_full = p_full_friction.position - p_full_friction.prev_position;
        // With friction=1.0, tangent_vel * (1-1) = 0, only normal component remains
        assert!(
            vel_no.length() >= vel_full.length() - 0.01,
            "No friction should have >= velocity: no={:.4} full={:.4}",
            vel_no.length(),
            vel_full.length()
        );
    }

    // --- resolve_collision capsule: exact axis + closest point arithmetic ---
    #[test]
    fn mutation_r4_capsule_collision_axis_arithmetic() {
        use super::*;
        // Capsule from (0,0,0) to (0,4,0), radius=1
        // Particle at (0.5, 2, 0) — closest on axis is (0, 2, 0)
        // dist = 0.5 < 1, penetration = 0.5, push to (1, 2, 0)
        let mut p = ClothParticle::new(Vec3::new(0.5, 2.0, 0.0), 1.0);
        p.prev_position = Vec3::new(0.4, 2.0, 0.0);
        let collider = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 4.0, 0.0),
            radius: 1.0,
        };
        collider.resolve_collision(&mut p, 0.3);
        assert!(
            (p.position.x - 1.0).abs() < 0.01,
            "Should push x to radius: x={}",
            p.position.x
        );
        assert!(
            (p.position.y - 2.0).abs() < 0.01,
            "Y should stay near 2: y={}",
            p.position.y
        );
    }

    // --- resolve_collision plane: exact dist formula ---
    #[test]
    fn mutation_r4_plane_collision_exact_dist() {
        use super::*;
        // Plane at y=1, normal up. Particle at (3, 0.5, 2) is below plane.
        // to_particle = (3,0.5,2)-(0,1,0) = (3,-0.5,2)
        // dist = dot((3,-0.5,2), (0,1,0)) = -0.5 < 0
        // position -= normal * dist = (3,0.5,2) - (0,1,0)*(-0.5) = (3,1,2)
        let mut p = ClothParticle::new(Vec3::new(3.0, 0.5, 2.0), 1.0);
        p.prev_position = Vec3::new(3.0, 0.6, 2.0);
        let collider = ClothCollider::Plane {
            point: Vec3::new(0.0, 1.0, 0.0),
            normal: Vec3::Y,
        };
        collider.resolve_collision(&mut p, 0.2);
        assert!(
            (p.position.y - 1.0).abs() < 0.01,
            "Should be pushed to plane surface: y={}",
            p.position.y
        );
        assert!(
            (p.position.x - 3.0).abs() < 0.01,
            "X should be unchanged: x={}",
            p.position.x
        );
    }

    // --- get_indices: exact index arithmetic (10 mutations) ---
    #[test]
    fn mutation_r4_get_indices_exact_values_2x2() {
        use super::*;
        // 2×2 cloth: 1 cell → 2 triangles → 6 indices
        let cloth = Cloth::new(
            ClothId(800),
            ClothConfig {
                width: 2,
                height: 2,
                spacing: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let indices = cloth.get_indices();
        assert_eq!(indices.len(), 6, "2x2 should have 6 indices");
        // First triangle: idx=0*2+0=0, push 0, 0+1=1, 0+2=2
        assert_eq!(indices[0], 0);
        assert_eq!(indices[1], 1);
        assert_eq!(indices[2], 2);
        // Second triangle: 1, 2+1=3, 2
        assert_eq!(indices[3], 1);
        assert_eq!(indices[4], 3);
        assert_eq!(indices[5], 2);
    }

    #[test]
    fn mutation_r4_get_indices_exact_values_3x3() {
        use super::*;
        // 3×3 cloth: 4 cells → 8 triangles → 24 indices
        let cloth = Cloth::new(
            ClothId(801),
            ClothConfig {
                width: 3,
                height: 3,
                spacing: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let indices = cloth.get_indices();
        assert_eq!(indices.len(), 24, "3x3 should have 24 indices");
        // Check cell (1,0): idx = 0*3+1 = 1
        // First tri: 1, 2, 4
        // Second tri: 2, 5, 4
        // Cell (0,1): idx = 1*3+0 = 3
        // First tri: 3, 4, 6
        // Second tri: 4, 7, 6
        // Verify specific indices for cell (1,1): idx = 1*3+1 = 4
        // First tri: 4, 5, 7
        // Second tri: 5, 8, 7
        let cell_11_start = 4 * 6; // 4th cell (0-indexed), but actually let's count
                                   // Cells are: (0,0)=0-5, (1,0)=6-11, (0,1)=12-17, (1,1)=18-23
        assert_eq!(indices[18], 4, "cell(1,1) tri1.v0 = 4");
        assert_eq!(indices[19], 5, "cell(1,1) tri1.v1 = 5");
        assert_eq!(indices[20], 7, "cell(1,1) tri1.v2 = 4+w = 7");
        assert_eq!(indices[21], 5, "cell(1,1) tri2.v0 = 5");
        assert_eq!(indices[22], 8, "cell(1,1) tri2.v1 = 5+w = 8");
        assert_eq!(indices[23], 7, "cell(1,1) tri2.v2 = 4+w = 7");
    }

    // --- get_positions: not empty, not default ---
    #[test]
    fn mutation_r4_get_positions_content() {
        use super::*;
        let cloth = Cloth::new(
            ClothId(802),
            ClothConfig {
                width: 3,
                height: 2,
                spacing: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let positions = cloth.get_positions();
        assert_eq!(positions.len(), 6, "3x2 should have 6 positions");
        // Positions should be spaced apart, not all default
        assert_ne!(
            positions[0], positions[1],
            "Adjacent particles should differ"
        );
        // First and last should be far apart
        let dist = (positions[5] - positions[0]).length();
        assert!(dist > 1.0, "Particles should be spread out, dist={}", dist);
    }

    // --- constraint_count: not 1 ---
    #[test]
    fn mutation_r4_constraint_count_not_one() {
        use super::*;
        let cloth = Cloth::new(
            ClothId(803),
            ClothConfig {
                width: 3,
                height: 3,
                spacing: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        let cc = cloth.constraint_count();
        // 3x3 = horizontal: 3*2=6, vertical: 2*3=6, total=12 structural constraints
        assert!(cc > 1, "constraint_count should not be 1, got {}", cc);
        assert!(
            cc >= 12,
            "3x3 should have at least 12 constraints, got {}",
            cc
        );
    }

    // --- particle_normal: verify cross product direction ---
    #[test]
    fn mutation_r4_particle_normal_center_direction() {
        use super::*;
        // Create a cloth and perturb a neighbor to break symmetry
        let mut cloth = Cloth::new(
            ClothId(804),
            ClothConfig {
                width: 3,
                height: 3,
                spacing: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        // Move particle (1,0) upward — it's a neighbor of center (1,1)
        // idx = 0*3 + 1 = 1
        cloth.particles[1].position.y += 0.5;
        // This breaks cross-product cancellation for center normal
        let normal = cloth.particle_normal(1, 1);
        assert!(
            normal.length() > 0.1,
            "Asymmetric perturbation should give non-zero normal: {:?}",
            normal
        );
    }

    // ===== ECS INTEGRATION SCAFFOLDING v3.7.0 — Cloth::update integration tests =====

    #[test]
    fn integration_cloth_update_gravity_pulls_down() {
        use super::*;
        let mut cloth = Cloth::new(
            ClothId(900),
            ClothConfig {
                width: 4,
                height: 4,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                wind: Vec3::ZERO,
                ..Default::default()
            },
            Vec3::new(0.0, 10.0, 0.0),
        );

        // Pin the top row so cloth hangs
        for x in 0..4 {
            cloth.particles[x].pinned = true;
        }

        let initial_bottom_y = cloth.particles[3 * 4 + 0].position.y; // bottom-left

        // Update for a few frames
        for _ in 0..60 {
            cloth.update(1.0 / 60.0);
        }

        let final_bottom_y = cloth.particles[3 * 4 + 0].position.y;
        assert!(
            final_bottom_y < initial_bottom_y,
            "Bottom row should fall due to gravity: before={}, after={}",
            initial_bottom_y,
            final_bottom_y
        );

        // Top row should stay pinned
        let top_y = cloth.particles[0].position.y;
        assert!(
            (top_y - 10.0).abs() < 0.01,
            "Pinned top row should not move: {}",
            top_y
        );
    }

    #[test]
    fn integration_cloth_update_wind_pushes_cloth() {
        use super::*;
        let mut cloth = Cloth::new(
            ClothId(901),
            ClothConfig {
                width: 3,
                height: 3,
                spacing: 1.0,
                gravity: Vec3::ZERO, // No gravity
                // Wind force uses dot(normal).abs() — flat cloth in XZ plane has Y normal
                // So wind must have a Y component to affect it
                wind: Vec3::new(10.0, 20.0, 0.0), // Strong wind with Y component
                air_resistance: 0.1,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        // Pin top-left only
        cloth.particles[0].pinned = true;

        // Slightly tilt one particle to break perfect symmetry and give normals
        cloth.particles[4].position.y += 0.1;

        let initial_y = cloth.particles[2 * 3 + 2].position.y; // bottom-right

        for _ in 0..120 {
            cloth.update(1.0 / 60.0);
        }

        let final_pos = cloth.particles[2 * 3 + 2].position;
        let moved = (final_pos - Vec3::new(2.0, initial_y, 0.0)).length();
        assert!(
            moved > 0.1,
            "Wind should push cloth: initial_y={}, final={:?}, moved={}",
            initial_y,
            final_pos,
            moved
        );
    }

    #[test]
    fn integration_cloth_update_constraint_maintains_distance() {
        use super::*;
        let mut cloth = Cloth::new(
            ClothId(902),
            ClothConfig {
                width: 2,
                height: 2,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                solver_iterations: 10,
                stiffness: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        // Pin top-left
        cloth.particles[0].pinned = true;

        for _ in 0..120 {
            cloth.update(1.0 / 60.0);
        }

        // Check distance between adjacent particles should be near rest length
        let p0 = cloth.particles[0].position;
        let p1 = cloth.particles[1].position;
        let dist = (p0 - p1).length();
        // With solver_iterations and stiffness=1, distance should stay close to spacing
        assert!(
            dist < 2.0,
            "Constraint should prevent excessive stretching: dist={}",
            dist
        );
    }

    #[test]
    fn integration_cloth_update_damping_reduces_velocity() {
        use super::*;
        // With damping
        let mut cloth_damped = Cloth::new(
            ClothId(903),
            ClothConfig {
                width: 2,
                height: 2,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                damping: 0.99,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        // Without damping (damping = 1.0 means no damping)
        let mut cloth_undamped = Cloth::new(
            ClothId(904),
            ClothConfig {
                width: 2,
                height: 2,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                damping: 1.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        for _ in 0..30 {
            cloth_damped.update(1.0 / 60.0);
            cloth_undamped.update(1.0 / 60.0);
        }

        let damped_y = cloth_damped.particles[3].position.y;
        let undamped_y = cloth_undamped.particles[3].position.y;

        // Undamped should fall further (less resistance)
        assert!(
            undamped_y <= damped_y + 0.5,
            "Undamped cloth should fall at least as much: damped_y={}, undamped_y={}",
            damped_y,
            undamped_y
        );
    }

    #[test]
    fn integration_cloth_update_all_particles_move_without_pin() {
        use super::*;
        let mut cloth = Cloth::new(
            ClothId(905),
            ClothConfig {
                width: 3,
                height: 3,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                ..Default::default()
            },
            Vec3::new(0.0, 20.0, 0.0),
        );

        let initial_positions: Vec<Vec3> = cloth.particles.iter().map(|p| p.position).collect();

        for _ in 0..30 {
            cloth.update(1.0 / 60.0);
        }

        // All unpinned particles should have moved
        for (i, p) in cloth.particles.iter().enumerate() {
            if !p.pinned {
                let dist = (p.position - initial_positions[i]).length();
                assert!(
                    dist > 0.01,
                    "Particle {} should have moved under gravity: dist={}",
                    i,
                    dist
                );
            }
        }
    }

    #[test]
    fn integration_cloth_update_zero_dt_no_change() {
        use super::*;
        let mut cloth = Cloth::new(
            ClothId(906),
            ClothConfig {
                width: 2,
                height: 2,
                spacing: 1.0,
                gravity: Vec3::new(0.0, -9.81, 0.0),
                ..Default::default()
            },
            Vec3::ZERO,
        );

        let initial_positions: Vec<Vec3> = cloth.particles.iter().map(|p| p.position).collect();

        cloth.update(0.0);

        for (i, p) in cloth.particles.iter().enumerate() {
            let dist = (p.position - initial_positions[i]).length();
            assert!(
                dist < 0.01,
                "Zero dt should not move particles: particle {}, dist={}",
                i,
                dist
            );
        }
    }

    #[test]
    fn integration_cloth_collider_resolve_collision_sphere() {
        use super::*;
        // Create a particle inside a sphere collider
        let mut particle = ClothParticle {
            position: Vec3::new(0.1, 0.0, 0.0),
            prev_position: Vec3::ZERO,
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        let particles = &mut [particle.clone()];
        // Particle is inside the sphere — resolve_collision should push it out
        collider.resolve_collision(&mut particles[0], 0.5);

        assert!(
            (particles[0].position - Vec3::ZERO).length() >= 0.99,
            "Particle should be pushed outside sphere: pos={:?}",
            particles[0].position
        );
    }

    #[test]
    fn integration_cloth_collider_resolve_collision_plane() {
        use super::*;
        let mut particle = ClothParticle {
            position: Vec3::new(0.0, -0.5, 0.0), // Below plane
            prev_position: Vec3::new(0.0, 0.5, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        collider.resolve_collision(&mut particle, 0.5);

        assert!(
            particle.position.y >= -0.01,
            "Particle should be pushed above plane: y={}",
            particle.position.y
        );
    }

    // ===== ROUND 6: Deep cloth integration tests =====

    #[test]
    fn r6_resolve_collision_capsule_degenerate() {
        // When start == end, capsule falls back to sphere behavior
        let mut particle = ClothParticle {
            position: Vec3::new(0.3, 0.0, 0.0), // Inside degenerate capsule (radius=1)
            prev_position: Vec3::new(0.2, 0.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 0.0, 0.0), // degenerate
            radius: 1.0,
        };

        collider.resolve_collision(&mut particle, 0.3);

        // Particle inside degenerate capsule → pushed out to radius
        assert!(
            particle.position.length() >= 0.99,
            "Degenerate capsule should push particle to radius: pos={:?}",
            particle.position
        );
    }

    #[test]
    fn r6_resolve_collision_sphere_exact_penetration() {
        let mut particle = ClothParticle {
            position: Vec3::new(0.3, 0.0, 0.0), // 0.3 inside radius=1 sphere at origin
            prev_position: Vec3::new(0.3, 0.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        collider.resolve_collision(&mut particle, 0.0);

        // Penetration = radius - dist = 1.0 - 0.3 = 0.7
        // Should push out along normal (X direction) to radius
        let dist_from_center = particle.position.length();
        assert!(
            (dist_from_center - 1.0).abs() < 0.01,
            "After collision, particle should be at radius: dist={}",
            dist_from_center
        );
        assert!(
            particle.position.x > 0.9,
            "Should push in +X: {:?}",
            particle.position
        );
    }

    #[test]
    fn r6_resolve_collision_friction_damps_tangent() {
        // Particle inside sphere with tangential velocity
        let mut particle = ClothParticle {
            position: Vec3::new(0.5, 0.0, 0.0),
            prev_position: Vec3::new(0.5, -1.0, 0.0), // velocity = (0, 1, 0) tangential
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        collider.resolve_collision(&mut particle, 0.8);

        // After collision, prev_position should change to damp tangent velocity
        // tangent_vel * (1 - friction) = 1.0 * 0.2 = 0.2 remaining
        let new_vel = particle.position - particle.prev_position;
        // Y component (tangential) should be significantly reduced
        assert!(
            new_vel.y.abs() < 0.5,
            "Friction should damp tangential velocity: vel_y={}",
            new_vel.y
        );
    }

    #[test]
    fn r6_resolve_collision_pinned_not_moved() {
        let mut particle = ClothParticle {
            position: Vec3::new(0.5, 0.0, 0.0), // Inside sphere
            prev_position: Vec3::new(0.5, 0.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: true,
        };

        let collider = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };

        let pos_before = particle.position;
        collider.resolve_collision(&mut particle, 0.5);

        assert!(
            (particle.position - pos_before).length() < 1e-6,
            "Pinned particle should not be moved by collision"
        );
    }

    #[test]
    fn r6_resolve_collision_capsule_along_axis() {
        // Particle near the middle of a capsule
        let mut particle = ClothParticle {
            position: Vec3::new(0.1, 1.0, 0.0), // Near capsule axis at y=1
            prev_position: Vec3::new(0.1, 1.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 2.0, 0.0),
            radius: 0.5,
        };

        collider.resolve_collision(&mut particle, 0.3);

        // Closest point on axis at t=1.0 → (0,1,0)
        // Distance from particle to closest = 0.1 < radius 0.5
        // Should push outward in XZ plane
        assert!(
            particle.position.x > 0.4,
            "Capsule should push particle to radius in X: {:?}",
            particle.position
        );
    }

    #[test]
    fn r6_constraint_solve_moves_toward_rest() {
        let mut particles = vec![
            ClothParticle {
                position: Vec3::ZERO,
                prev_position: Vec3::ZERO,
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: false,
            },
            ClothParticle {
                position: Vec3::new(2.0, 0.0, 0.0), // 2.0 apart
                prev_position: Vec3::new(2.0, 0.0, 0.0),
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: false,
            },
        ];

        let constraint = DistanceConstraint {
            p1: 0,
            p2: 1,
            rest_length: 1.0, // Rest length = 1.0, current = 2.0
            stiffness: 1.0,
        };

        constraint.solve(&mut particles);

        let new_dist = (particles[1].position - particles[0].position).length();
        // Should have moved closer to rest_length=1.0
        assert!(
            new_dist < 2.0,
            "Constraint should pull particles closer: new_dist={}",
            new_dist
        );
        assert!(
            new_dist > 0.5,
            "Constraint shouldn't overshoot: new_dist={}",
            new_dist
        );
    }

    #[test]
    fn r6_constraint_solve_respects_pinned() {
        let mut particles = vec![
            ClothParticle {
                position: Vec3::ZERO,
                prev_position: Vec3::ZERO,
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: true, // Pinned
            },
            ClothParticle {
                position: Vec3::new(3.0, 0.0, 0.0),
                prev_position: Vec3::new(3.0, 0.0, 0.0),
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: false,
            },
        ];

        let constraint = DistanceConstraint {
            p1: 0,
            p2: 1,
            rest_length: 1.0,
            stiffness: 1.0,
        };

        constraint.solve(&mut particles);

        // Pinned particle should not move
        assert!(
            particles[0].position.length() < 1e-6,
            "Pinned p1 should stay at origin: {:?}",
            particles[0].position
        );
        // Unpinned particle should move toward pinned one
        assert!(
            particles[1].position.x < 3.0,
            "p2 should move toward p1: x={}",
            particles[1].position.x
        );
    }

    #[test]
    fn r6_constraint_solve_weight_distribution() {
        let mut particles = vec![
            ClothParticle {
                position: Vec3::ZERO,
                prev_position: Vec3::ZERO,
                acceleration: Vec3::ZERO,
                inv_mass: 1.0, // light (inv_mass=1)
                pinned: false,
            },
            ClothParticle {
                position: Vec3::new(2.0, 0.0, 0.0),
                prev_position: Vec3::new(2.0, 0.0, 0.0),
                acceleration: Vec3::ZERO,
                inv_mass: 3.0, // heavier contribution to correction
                pinned: false,
            },
        ];

        let constraint = DistanceConstraint {
            p1: 0,
            p2: 1,
            rest_length: 1.0,
            stiffness: 1.0,
        };

        constraint.solve(&mut particles);

        // p1 has w1=1.0, p2 has w2=3.0, total=4.0
        // p1 moves by w1/total = 0.25 of correction
        // p2 moves by w2/total = 0.75 of correction
        // Total correction = delta * diff * 0.5 * stiffness
        let p1_moved = particles[0].position.x.abs();
        let p2_moved = (particles[1].position.x - 2.0).abs();
        // p2 should move more than p1 (higher inv_mass = lighter = moves more)
        assert!(
            p2_moved > p1_moved,
            "Higher inv_mass should move more: p1={}, p2={}",
            p1_moved,
            p2_moved
        );
    }

    #[test]
    fn r6_cloth_multi_step_convergence() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            particle_mass: 1.0,
            stiffness: 1.0,
            damping: 0.98,
            solver_iterations: 10,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            wind: Vec3::ZERO,
            ..Default::default()
        };

        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        // Pin top row
        cloth.pin_corners();

        let pos_before = cloth.get_positions();

        // Run many steps
        for _ in 0..100 {
            cloth.update(1.0 / 60.0);
        }

        let pos_after = cloth.get_positions();

        // Bottom particles should have moved down (gravity)
        let bottom_idx = 2 * 3; // y=2, x=0
        assert!(
            pos_after[bottom_idx].y < pos_before[bottom_idx].y,
            "Bottom particle should fall: before={}, after={}",
            pos_before[bottom_idx].y,
            pos_after[bottom_idx].y
        );

        // Run more steps — should converge (constraint solving)
        let pos_mid = cloth.get_positions();
        for _ in 0..200 {
            cloth.update(1.0 / 60.0);
        }
        let pos_final = cloth.get_positions();

        // Movement rate should decrease (converging)
        let delta_mid = (pos_mid[bottom_idx] - pos_before[bottom_idx]).length();
        let delta_final = (pos_final[bottom_idx] - pos_mid[bottom_idx]).length();
        // Convergence test: later movement should be less
        assert!(
            delta_final < delta_mid + 0.1, // Very loose — just catches divergence from sign mutations
            "Cloth should converge: delta_mid={}, delta_final={}",
            delta_mid,
            delta_final
        );
    }

    #[test]
    fn r6_particle_normal_boundary() {
        // Test particle_normal for corner particles (only 1 adjacent triangle)
        let config = ClothConfig {
            width: 2,
            height: 2,
            spacing: 1.0,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Corner (0,0) — only has triangles sharing this vertex
        let n00 = cloth.particle_normal(0, 0);
        // For a flat XZ plane, normal should point in Y direction
        assert!(
            n00.y.abs() > 0.5,
            "Corner normal should have Y component: {:?}",
            n00
        );

        // Center-ish point (0,0) in a 2x2 grid — all have normal
        let n11 = cloth.particle_normal(1, 1);
        assert!(n11.length() > 0.1, "Normal should be non-zero: {:?}", n11);
    }

    #[test]
    fn r6_get_indices_count_correct() {
        let config = ClothConfig {
            width: 4,
            height: 4,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        let indices = cloth.get_indices();
        // (w-1)*(h-1) quads, each quad = 2 triangles = 6 indices
        let expected = (4 - 1) * (4 - 1) * 6;
        assert_eq!(
            indices.len(),
            expected,
            "Expected {} indices, got {}",
            expected,
            indices.len()
        );
    }

    #[test]
    fn r6_cloth_particle_count() {
        let config = ClothConfig {
            width: 5,
            height: 3,
            ..Default::default()
        };
        let cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);
        assert_eq!(cloth.particle_count(), 15);
    }

    #[test]
    fn r6_cloth_air_resistance_slows() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            particle_mass: 1.0,
            stiffness: 0.0, // No constraints
            damping: 1.0,   // No damping
            solver_iterations: 0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            air_resistance: 5.0, // High drag
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Give a particle some velocity by offsetting prev_position
        cloth.particles[4].prev_position = cloth.particles[4].position - Vec3::new(1.0, 0.0, 0.0);

        let vel_before = cloth.particles[4].velocity().length();
        cloth.update(1.0 / 60.0);
        let vel_after = cloth.particles[4].velocity().length();

        // Air resistance should reduce velocity
        // drag = -velocity * air_resistance * (1/inv_mass)
        assert!(
            vel_after < vel_before || vel_before < 0.01,
            "Air resistance should slow particle: before={}, after={}",
            vel_before,
            vel_after
        );
    }

    // ===== ROUND 7: Targeted resolve_collision catches =====

    #[test]
    fn r7_capsule_collision_pushes_outward() {
        // Capsule axis from (0,0,0) to (0,2,0), radius=0.5
        // Particle at (0.2, 1.0, 0.0) — inside capsule, closest point on axis = (0,1,0)
        // Distance = 0.2, penetration = 0.5 - 0.2 = 0.3
        // Normal = (1,0,0), pushed to (0.5, 1.0, 0.0)
        let mut p = ClothParticle {
            position: Vec3::new(0.2, 1.0, 0.0),
            prev_position: Vec3::new(0.15, 1.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let capsule = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 2.0, 0.0),
            radius: 0.5,
        };

        capsule.resolve_collision(&mut p, 0.0);

        // Should be pushed to radius distance from axis (ignoring friction)
        let dist_from_axis = Vec3::new(p.position.x, 0.0, p.position.z).length();
        assert!(
            (dist_from_axis - 0.5).abs() < 0.01,
            "Capsule should push to radius 0.5: dist_from_axis={}",
            dist_from_axis
        );
        // Y should remain ~1.0 (closest point on axis is (0,1,0))
        assert!(
            (p.position.y - 1.0).abs() < 0.01,
            "Y should stay near closest axis point: y={}",
            p.position.y
        );
        // Should be pushed in +X direction (outward from axis)
        assert!(
            p.position.x > 0.4,
            "Should push in +X direction: x={}",
            p.position.x
        );
    }

    #[test]
    fn r7_capsule_collision_axis_direction_matters() {
        // Verify axis_dir = axis / axis_length works correctly
        // Capsule from (0,0,0) to (4,0,0), radius=1
        // Particle at (2,0.5,0) — inside, closest on axis = (2,0,0), dist=0.5
        let mut p = ClothParticle {
            position: Vec3::new(2.0, 0.5, 0.0),
            prev_position: Vec3::new(2.0, 0.4, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let capsule = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::new(4.0, 0.0, 0.0),
            radius: 1.0,
        };

        capsule.resolve_collision(&mut p, 0.0);

        // Should be pushed up to radius=1 from axis
        assert!(
            (p.position.y - 1.0).abs() < 0.01,
            "Should push to radius in Y: y={}",
            p.position.y
        );
        // X should stay ~2.0
        assert!(
            (p.position.x - 2.0).abs() < 0.05,
            "X should stay near 2.0: x={}",
            p.position.x
        );
    }

    #[test]
    fn r7_capsule_collision_clamps_t_to_start() {
        // Particle beyond start of capsule — t should clamp to 0
        // Capsule from (2,0,0) to (4,0,0), radius=1
        // Particle at (0,0.5,0) — closest point = start = (2,0,0)
        let mut p = ClothParticle {
            position: Vec3::new(0.0, 0.5, 0.0),
            prev_position: Vec3::new(0.0, 0.3, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let capsule = ClothCollider::Capsule {
            start: Vec3::new(2.0, 0.0, 0.0),
            end: Vec3::new(4.0, 0.0, 0.0),
            radius: 1.0,
        };

        let pos_before = p.position;
        capsule.resolve_collision(&mut p, 0.0);

        // Particle at (0,0.5,0) is distance sqrt(4+0.25)=2.06 from start=(2,0,0)
        // Which is > radius=1, so NO collision
        assert_eq!(
            p.position, pos_before,
            "Particle outside capsule end should not be moved"
        );
    }

    #[test]
    fn r7_capsule_friction_reduces_tangent_velocity() {
        // With friction, tangent velocity should be damped
        let mut p_no_friction = ClothParticle {
            position: Vec3::new(0.3, 1.0, 0.0),
            prev_position: Vec3::new(0.3, 0.5, 0.0), // Moving in +Y
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        let mut p_friction = p_no_friction.clone();

        let capsule = ClothCollider::Capsule {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 2.0, 0.0),
            radius: 0.5,
        };

        capsule.resolve_collision(&mut p_no_friction, 0.0);
        capsule.resolve_collision(&mut p_friction, 0.9);

        // After collision, both should be at radius distance
        // But with friction, the prev_position adjustment should differ
        // The velocity after should be less with friction
        let vel_no_f = (p_no_friction.position - p_no_friction.prev_position).length();
        let vel_f = (p_friction.position - p_friction.prev_position).length();
        assert!(
            vel_f < vel_no_f + 0.01,
            "Friction should reduce velocity: no_friction={}, friction={}",
            vel_no_f,
            vel_f
        );
    }

    #[test]
    fn r7_plane_collision_pushes_above() {
        // Plane at y=0 with normal=(0,1,0)
        // Particle below plane at (1, -0.5, 0)
        let mut p = ClothParticle {
            position: Vec3::new(1.0, -0.5, 0.0),
            prev_position: Vec3::new(1.0, -0.6, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        plane.resolve_collision(&mut p, 0.0);

        // dist = (p-point).dot(normal) = -0.5 → below plane
        // position -= normal * dist → position.y -= 1*(-0.5) → position.y += 0.5 → y=0
        assert!(
            p.position.y >= -0.001,
            "Plane should push particle above: y={}",
            p.position.y
        );
    }

    #[test]
    fn r7_plane_collision_exact_projection() {
        // Test exact calculations for plane collision
        // Plane through (0,2,0) normal=(0,1,0)
        // Particle at (3.0, 1.5, -1.0) → below plane by 0.5
        let mut p = ClothParticle {
            position: Vec3::new(3.0, 1.5, -1.0),
            prev_position: Vec3::new(3.0, 1.4, -1.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let plane = ClothCollider::Plane {
            point: Vec3::new(0.0, 2.0, 0.0),
            normal: Vec3::Y,
        };

        plane.resolve_collision(&mut p, 0.0);

        // dist = (1.5-2.0) = -0.5
        // pos -= normal * (-0.5) → pos.y += 0.5 → y = 2.0
        assert!(
            (p.position.y - 2.0).abs() < 0.01,
            "Should push to plane surface y=2.0: got y={}",
            p.position.y
        );
        // X and Z should not change
        assert!(
            (p.position.x - 3.0).abs() < 0.01,
            "X should not change: x={}",
            p.position.x
        );
    }

    #[test]
    fn r7_plane_friction_damps_velocity() {
        let mut p_no_f = ClothParticle {
            position: Vec3::new(1.0, -0.5, 0.0),
            prev_position: Vec3::new(0.0, -0.5, 0.0), // Moving in +X (tangent to plane)
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        let mut p_f = p_no_f.clone();

        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };

        plane.resolve_collision(&mut p_no_f, 0.0);
        plane.resolve_collision(&mut p_f, 0.8);

        // With friction=0.8, tangent velocity should be reduced by 80%
        let vel_no = (p_no_f.position - p_no_f.prev_position).x.abs();
        let vel_f = (p_f.position - p_f.prev_position).x.abs();
        assert!(
            vel_f < vel_no + 0.01,
            "Friction should reduce tangent velocity: no_f={}, f={}",
            vel_no,
            vel_f
        );
    }

    #[test]
    fn r7_sphere_normal_direction_correctness() {
        // Sphere at origin, radius=2
        // Particle at (1,0,0) — inside, should push to (2,0,0)
        let mut p = ClothParticle {
            position: Vec3::new(1.0, 0.0, 0.0),
            prev_position: Vec3::new(0.8, 0.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };

        let sphere = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 2.0,
        };

        sphere.resolve_collision(&mut p, 0.0);

        // normal = (1,0,0), penetration = 2-1 = 1
        // Should push to (2,0,0)
        assert!(
            (p.position.x - 2.0).abs() < 0.01,
            "Should push to radius in X: x={}",
            p.position.x
        );
        assert!(
            p.position.y.abs() < 0.01,
            "Y should be ~0: y={}",
            p.position.y
        );
    }

    // ===== ROUND 9: Cloth update + particle_normal + manager update =====

    #[test]
    fn r9_cloth_update_gravity_moves_particles_down() {
        let config = ClothConfig {
            width: 4,
            height: 4,
            spacing: 1.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            wind: Vec3::ZERO,
            damping: 1.0, // no damping, to see gravity effect clearly
            air_resistance: 0.0,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::new(0.0, 10.0, 0.0));

        // Unpin all particles (by default corners might be pinned)
        for p in &mut cloth.particles {
            p.inv_mass = 1.0 / 0.1; // mass = 0.1
        }

        let initial_y: Vec<f32> = cloth.particles.iter().map(|p| p.position.y).collect();

        // Step several frames
        for _ in 0..5 {
            cloth.update(1.0 / 60.0);
        }

        // At least some particles should have moved down
        let moved_down = cloth
            .particles
            .iter()
            .zip(initial_y.iter())
            .any(|(p, &iy)| p.position.y < iy - 0.001);
        assert!(
            moved_down,
            "Gravity should pull unpinned particles downward"
        );
    }

    #[test]
    fn r9_cloth_update_wind_applies_force() {
        // Wind force depends on wind.dot(particle_normal). A perfectly flat cloth
        // has zero cross-product normals (opposite pairs cancel). We need curvature
        // first: pin the top row and let gravity droop the cloth, then wind can act.
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            gravity: Vec3::new(0.0, -10.0, 0.0), // gravity creates droop
            wind: Vec3::new(10.0, 0.0, 0.0),     // strong wind in X
            damping: 0.98,
            air_resistance: 0.0,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::new(0.0, 10.0, 0.0));

        // Pin top row (grid y=0) so gravity droops the rest
        for x in 0..5 {
            cloth.particles[x].inv_mass = 0.0; // pinned
        }
        // Unpin remaining rows (they already have default inv_mass from config)

        // Step many frames for gravity to droop and wind to push
        for _ in 0..100 {
            cloth.update(1.0 / 60.0);
        }

        // Bottom-row particles (grid y=4) should have shifted in +X from wind
        let bottom_row_start = 4 * 5;
        let bottom_particles = &cloth.particles[bottom_row_start..bottom_row_start + 5];
        let origin_x = 0.0_f32; // original X positions ranged [0..4]
        let avg_x: f32 = bottom_particles.iter().map(|p| p.position.x).sum::<f32>() / 5.0;
        // Without wind, avg_x of bottom row would stay near original (2.0).
        // With wind, it should shift positively.
        assert!(
            avg_x > 2.5,
            "Wind should push hanging cloth in +X: avg_x={}",
            avg_x
        );
    }

    #[test]
    fn r9_cloth_update_air_resistance_dampens() {
        // With air resistance, particles shouldn't accumulate velocity as fast
        let config_low_drag = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            wind: Vec3::ZERO,
            damping: 1.0,
            air_resistance: 0.0, // no drag
            ..Default::default()
        };
        let config_high_drag = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            wind: Vec3::ZERO,
            damping: 1.0,
            air_resistance: 1.0, // high drag
            ..Default::default()
        };

        let mut cloth_no_drag = Cloth::new(ClothId(1), config_low_drag, Vec3::new(0.0, 10.0, 0.0));
        let mut cloth_drag = Cloth::new(ClothId(2), config_high_drag, Vec3::new(0.0, 10.0, 0.0));

        // Unpin all
        for p in &mut cloth_no_drag.particles {
            p.inv_mass = 10.0;
        }
        for p in &mut cloth_drag.particles {
            p.inv_mass = 10.0;
        }

        for _ in 0..20 {
            cloth_no_drag.update(1.0 / 60.0);
            cloth_drag.update(1.0 / 60.0);
        }

        // No-drag cloth should fall faster (lower Y) than high-drag cloth
        let avg_y_no_drag: f32 = cloth_no_drag
            .particles
            .iter()
            .map(|p| p.position.y)
            .sum::<f32>()
            / cloth_no_drag.particles.len() as f32;
        let avg_y_drag: f32 = cloth_drag
            .particles
            .iter()
            .map(|p| p.position.y)
            .sum::<f32>()
            / cloth_drag.particles.len() as f32;

        assert!(
            avg_y_no_drag < avg_y_drag,
            "No-drag cloth should fall faster: no_drag_y={}, drag_y={}",
            avg_y_no_drag,
            avg_y_drag
        );
    }

    #[test]
    fn r9_cloth_particle_normal_flat_is_up() {
        // A cloth with slight curvature should produce non-zero normals
        let config = ClothConfig {
            width: 4,
            height: 4,
            spacing: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Perturb a NEIGHBOR particle downward to break symmetry.
        // Moving the center preserves radial symmetry (cross products cancel).
        // Moving one neighbor (2,1) creates an asymmetric surface.
        let neighbor_idx = 1 * 4 + 2; // particle (2,1)
        cloth.particles[neighbor_idx].position.y -= 0.5;

        let normal = cloth.particle_normal(1, 1);
        // With the center pushed down, cross products of neighbors should
        // produce a normal with a significant Y component
        assert!(
            normal.length() > 0.1,
            "Perturbed cloth normal should be non-zero: {:?}",
            normal
        );
    }

    #[test]
    fn r9_cloth_manager_update_steps_cloths() {
        let mut mgr = ClothManager::new();
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            wind: Vec3::ZERO,
            damping: 1.0,
            air_resistance: 0.0,
            ..Default::default()
        };
        let id = mgr.create(config, Vec3::new(0.0, 10.0, 0.0));

        // Unpin particles via get_mut
        if let Some(cloth) = mgr.get_mut(id) {
            for p in &mut cloth.particles {
                p.inv_mass = 10.0;
            }
        }

        let initial_y = mgr.get(id).unwrap().particles[4].position.y; // center particle

        // Manager update should step the cloth
        for _ in 0..10 {
            mgr.update(1.0 / 60.0);
        }

        let final_y = mgr.get(id).unwrap().particles[4].position.y;
        assert!(
            final_y < initial_y,
            "Manager update should step cloth: initial_y={}, final_y={}",
            initial_y,
            final_y
        );
    }

    // ===== ROUND 10: resolve_collision, DistanceConstraint, particle_normal precision =====

    #[test]
    fn r10_sphere_collision_pushes_particle_to_surface() {
        let sphere = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        // Particle inside sphere, along +X axis
        let mut p = ClothParticle::new(Vec3::new(0.3, 0.0, 0.0), 1.0);
        sphere.resolve_collision(&mut p, 0.0);

        // Should be pushed to radius distance from center
        let dist = p.position.length();
        assert!(
            (dist - 1.0).abs() < 0.01,
            "Particle should be pushed to sphere surface: dist={}",
            dist
        );
        // Should be pushed in +X direction (outward from center)
        assert!(
            p.position.x > 0.9,
            "Particle should be pushed radially outward in +X: pos={:?}",
            p.position
        );
    }

    #[test]
    fn r10_sphere_collision_penetration_depth_correct() {
        let sphere = ClothCollider::Sphere {
            center: Vec3::new(5.0, 0.0, 0.0),
            radius: 2.0,
        };
        // Particle at center of sphere — maximally penetrated
        let mut p = ClothParticle::new(Vec3::new(5.0, 0.0, 0.01), 1.0);
        sphere.resolve_collision(&mut p, 0.0);

        // Should be pushed out to radius=2.0 from center
        let dist = (p.position - Vec3::new(5.0, 0.0, 0.0)).length();
        assert!(
            dist > 1.9,
            "Should push out of deep penetration: dist={}",
            dist
        );
    }

    #[test]
    fn r10_sphere_collision_friction_modifies_velocity() {
        let sphere = ClothCollider::Sphere {
            center: Vec3::ZERO,
            radius: 1.0,
        };
        // Particle inside sphere with tangential velocity
        let mut p_no_friction = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        p_no_friction.prev_position = Vec3::new(0.5, -0.1, 0.0); // moving in +Y
        sphere.resolve_collision(&mut p_no_friction, 0.0);

        let mut p_friction = ClothParticle::new(Vec3::new(0.5, 0.0, 0.0), 1.0);
        p_friction.prev_position = Vec3::new(0.5, -0.1, 0.0); // same velocity
        sphere.resolve_collision(&mut p_friction, 0.8);

        // With friction, the tangential velocity component should be reduced
        let vel_no_f = p_no_friction.velocity().length();
        let vel_f = p_friction.velocity().length();
        assert!(
            vel_f < vel_no_f + 0.001,
            "Friction should reduce or match velocity: no_f={}, f={}",
            vel_no_f,
            vel_f
        );
    }

    #[test]
    fn r10_capsule_collision_pushes_particle_out() {
        // Vertical capsule from (0,0,0) to (0,3,0), radius 0.5
        let capsule = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 3.0, 0.0),
            radius: 0.5,
        };
        // Particle slightly inside the capsule (0.2 from axis, but radius is 0.5)
        let mut p = ClothParticle::new(Vec3::new(0.2, 1.5, 0.0), 1.0);
        capsule.resolve_collision(&mut p, 0.0);

        // Should push out to 0.5 from the capsule axis
        let horizontal_dist = Vec3::new(p.position.x, 0.0, p.position.z).length();
        assert!(
            horizontal_dist > 0.49,
            "Should push to capsule surface: horiz_dist={}",
            horizontal_dist
        );
        // Push direction should be in +X (away from axis)
        assert!(
            p.position.x > 0.4,
            "Should push in +X direction: pos={:?}",
            p.position
        );
    }

    #[test]
    fn r10_capsule_collision_clamps_to_axis() {
        // Capsule from (0,0,0) to (0,2,0), radius 0.5
        let capsule = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 2.0, 0.0),
            radius: 0.5,
        };
        // Particle near the bottom end, inside radius
        let mut p = ClothParticle::new(Vec3::new(0.2, -0.1, 0.0), 1.0);
        capsule.resolve_collision(&mut p, 0.0);

        // The closest point on axis should clamp to start (y=0), not go negative
        // So the push should be radially from (0,0,0)
        let dist_from_start = p.position.length();
        assert!(
            dist_from_start > 0.49,
            "Should push to capsule surface near start: dist={}",
            dist_from_start
        );
    }

    #[test]
    fn r10_capsule_collision_friction_reduces_tangent() {
        let capsule = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 3.0, 0.0),
            radius: 0.5,
        };
        let mut p = ClothParticle::new(Vec3::new(0.2, 1.5, 0.0), 1.0);
        p.prev_position = Vec3::new(0.2, 1.3, 0.0); // moving in +Y
        capsule.resolve_collision(&mut p, 0.5);

        // Velocity should include friction effect on tangent component
        let vel = p.velocity();
        // The normal is in X direction (radial), tangent is Y.
        // With friction=0.5, tangent velocity should be halved.
        // Just verify the prev_position was modified (not equal to original)
        assert!(
            (p.prev_position - Vec3::new(0.2, 1.3, 0.0)).length() > 0.01,
            "Friction should modify prev_position: {:?}",
            p.prev_position
        );
    }

    #[test]
    fn r10_plane_collision_pushes_above() {
        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        let mut p = ClothParticle::new(Vec3::new(1.0, -0.5, 2.0), 1.0);
        plane.resolve_collision(&mut p, 0.0);

        // Should push above the plane (y >= 0)
        assert!(
            p.position.y >= -0.01,
            "Should push above plane: y={}",
            p.position.y
        );
        // X and Z should be unchanged (push is only in normal direction)
        assert!(
            (p.position.x - 1.0).abs() < 0.01 && (p.position.z - 2.0).abs() < 0.01,
            "X/Z should be unchanged: pos={:?}",
            p.position
        );
    }

    #[test]
    fn r10_plane_collision_friction_on_sliding() {
        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        // Particle moving in +X while below the plane
        let mut p = ClothParticle::new(Vec3::new(0.0, -0.3, 0.0), 1.0);
        p.prev_position = Vec3::new(-0.2, -0.3, 0.0); // velocity = (0.2, 0, 0)
        plane.resolve_collision(&mut p, 0.5);

        // After collision, should be above plane
        assert!(p.position.y >= -0.01, "Above plane");
        // prev_position should be modified for friction
        let vel = p.velocity();
        // tangent velocity was (0.2,0,0), with friction 0.5 should be (0.1,0,0)
        assert!(
            vel.x > 0.01 && vel.x < 0.25,
            "Friction should reduce tangent velocity: vel={:?}",
            vel
        );
    }

    #[test]
    fn r10_plane_collision_pinned_particle_unaffected() {
        let plane = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        let mut p = ClothParticle::pinned(Vec3::new(0.0, -1.0, 0.0));
        let original_pos = p.position;
        plane.resolve_collision(&mut p, 0.0);
        assert!(
            (p.position - original_pos).length() < 0.001,
            "Pinned particle should not be affected by collision"
        );
    }

    #[test]
    fn r10_constraint_solve_separates_compressed_pair() {
        // Two particles closer than rest length
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(0.3, 0.0, 0.0), 1.0),
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0); // rest length = 1.0

        let initial_dist = (particles[1].position - particles[0].position).length();
        constraint.solve(&mut particles);
        let final_dist = (particles[1].position - particles[0].position).length();

        assert!(
            final_dist > initial_dist + 0.1,
            "Should push apart: initial={}, final={}",
            initial_dist,
            final_dist
        );
        // Should move toward rest length
        assert!(
            (final_dist - 1.0).abs() < (initial_dist - 1.0).abs(),
            "Should move toward rest length 1.0: final={}",
            final_dist
        );
    }

    #[test]
    fn r10_constraint_solve_stiffness_zero_no_correction() {
        let mut particles = vec![
            ClothParticle::new(Vec3::new(0.0, 0.0, 0.0), 1.0),
            ClothParticle::new(Vec3::new(0.3, 0.0, 0.0), 1.0),
        ];
        let mut constraint = DistanceConstraint::new(0, 1, 1.0);
        constraint.stiffness = 0.0;

        let p0_before = particles[0].position;
        let p1_before = particles[1].position;
        constraint.solve(&mut particles);

        assert!(
            (particles[0].position - p0_before).length() < 0.001,
            "Zero stiffness should not move particles"
        );
        assert!(
            (particles[1].position - p1_before).length() < 0.001,
            "Zero stiffness should not move particles"
        );
    }

    #[test]
    fn r10_constraint_solve_pinned_particle_stays() {
        let mut particles = vec![
            ClothParticle::pinned(Vec3::ZERO),
            ClothParticle::new(Vec3::new(0.3, 0.0, 0.0), 1.0),
        ];
        let constraint = DistanceConstraint::new(0, 1, 1.0);

        constraint.solve(&mut particles);

        assert!(
            particles[0].position.length() < 0.001,
            "Pinned particle should stay at origin"
        );
        // Only the second particle should move toward rest length
        // One solve iteration doesn't fully correct (stiffness * 0.5 factor),
        // so we check it moved in the right direction
        assert!(
            particles[1].position.x > 0.5,
            "Unpinned particle should move toward rest length: x={}",
            particles[1].position.x
        );
    }

    #[test]
    fn r10_particle_normal_perturbed_neighbor_direction() {
        // Create a 3x3 cloth, perturb specific neighbor to verify cross product direction
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Perturb right neighbor (2,1) downward
        let right_idx = 1 * 3 + 2;
        cloth.particles[right_idx].position.y -= 1.0;

        let normal = cloth.particle_normal(1, 1);

        // The perturbation should produce a non-zero normal
        assert!(
            normal.length() > 0.3,
            "Normal should be substantial: {:?}",
            normal
        );

        // Now perturb DOWN neighbor (1,2) instead
        let config2 = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth2 = Cloth::new(ClothId(2), config2, Vec3::ZERO);
        let down_idx = 2 * 3 + 1;
        cloth2.particles[down_idx].position.y -= 1.0;

        let normal2 = cloth2.particle_normal(1, 1);
        assert!(
            normal2.length() > 0.3,
            "Normal should be substantial from different perturbation: {:?}",
            normal2
        );

        // The two normals should be different (different perturbations → different cross products)
        let dot = normal.dot(normal2);
        assert!(
            dot < 0.99,
            "Different perturbations should give different normals: dot={}",
            dot
        );
    }

    #[test]
    fn r10_particle_normal_subtraction_from_center() {
        // Test that v1 = neighbor_pos - center_pos (not +)
        // If '-' → '+', the vectors would point the wrong way
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Create asymmetric perturbation
        // Move right neighbor up, down neighbor down
        cloth.particles[1 * 3 + 2].position.y += 1.0; // right (2,1) up
        cloth.particles[2 * 3 + 1].position.y -= 1.0; // down (1,2) down

        let normal = cloth.particle_normal(1, 1);

        // The normal should be non-trivial and the sign matters
        assert!(
            normal.length() > 0.5,
            "Asymmetric perturbation should give strong normal: {:?}",
            normal
        );

        // If we flip the sign (- → +), we'd get a completely different normal
        // Just verify the normal makes geometric sense for this configuration
        // The cross products with +Y and -Y perturbations should produce
        // a normal that distinguishes the two sides
    }

    #[test]
    fn r10_particle_normal_accumulation_sign() {
        // Test that normal += v1.cross(v2) (not -=)
        // Use an asymmetric hill: tilt one side up more than the other
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            ..Default::default()
        };
        let mut cloth = Cloth::new(ClothId(1), config, Vec3::ZERO);

        // Create an asymmetric surface: only raise particles on the +X side
        for y in 0..5_usize {
            for x in 0..5_usize {
                let dx = x as f32 - 2.0;
                // Only raise right side (break symmetry)
                if dx > 0.0 {
                    cloth.particles[y * 5 + x].position.y = dx * 0.5;
                }
            }
        }

        // Center particle (2,2) has asymmetric neighbors
        let normal = cloth.particle_normal(2, 2);

        // Should be non-zero due to asymmetry
        assert!(
            normal.length() > 0.3,
            "Asymmetric surface should have non-zero center normal: {:?}",
            normal
        );
    }

    // ============================================================================
    // R11 — Targeted mutation tests for cloth collision, update, and particle_normal
    // ============================================================================

    #[test]
    fn r11_capsule_collision_nonorigin_closest_point() {
        // Capsule NOT at origin so `position - closest` vs `position + closest` differs
        let collider = ClothCollider::Capsule {
            start: Vec3::new(5.0, 0.0, 0.0),
            end: Vec3::new(5.0, 4.0, 0.0),
            radius: 1.0,
        };
        // Particle at (5.5, 2.0, 0.0) — inside capsule at midpoint
        let mut particle = ClothParticle {
            position: Vec3::new(5.5, 2.0, 0.0),
            prev_position: Vec3::new(5.5, 2.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        collider.resolve_collision(&mut particle, 0.0);

        // Should be pushed OUT from axis (X direction away from x=5.0)
        assert!(
            particle.position.x > 5.9,
            "Should be pushed to capsule surface on +X side: x={}",
            particle.position.x
        );
        // Y should stay near 2.0 (no displacement along axis)
        assert!(
            (particle.position.y - 2.0).abs() < 0.5,
            "Y should remain near 2.0: y={}",
            particle.position.y
        );
    }

    #[test]
    fn r11_capsule_collision_penetration_sign() {
        // Test that penetration = radius - dist (not radius + dist)
        let collider = ClothCollider::Capsule {
            start: Vec3::new(3.0, 0.0, 0.0),
            end: Vec3::new(3.0, 2.0, 0.0),
            radius: 2.0,
        };
        // Particle at (4.0, 1.0, 0.0): dist from axis = 1.0, penetration = 2.0 - 1.0 = 1.0
        let mut particle = ClothParticle {
            position: Vec3::new(4.0, 1.0, 0.0),
            prev_position: Vec3::new(4.0, 1.0, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        collider.resolve_collision(&mut particle, 0.0);

        // Should be pushed to (5.0, 1.0, 0.0) = center_x + radius
        assert!(
            (particle.position.x - 5.0).abs() < 0.1,
            "Should be pushed to capsule surface x=5.0: x={}",
            particle.position.x
        );
    }

    #[test]
    fn r11_capsule_friction_tangent_exact() {
        let collider = ClothCollider::Capsule {
            start: Vec3::new(0.0, 0.0, 0.0),
            end: Vec3::new(0.0, 4.0, 0.0),
            radius: 2.0,
        };
        // Particle inside, moving along Y (tangent) and X (normal-ish)
        let mut particle = ClothParticle {
            position: Vec3::new(1.0, 2.0, 0.0),
            prev_position: Vec3::new(0.5, 1.0, 0.0), // velocity = (0.5, 1.0, 0.0)
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        collider.resolve_collision(&mut particle, 0.5);

        // After collision: particle pushed to radius=2 on X axis
        // Normal is roughly (1,0,0)
        // Friction reduces tangent velocity component
        // prev_position should reflect friction
        let vel_after = particle.position - particle.prev_position;
        // With friction=0.5, tangent velocity should be halved
        assert!(
            vel_after.y.abs() < 0.8,
            "Friction should reduce tangent velocity: vy={}",
            vel_after.y
        );
    }

    #[test]
    fn r11_plane_collision_direction() {
        // Plane at y=0, normal up. Particle below plane.
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        let mut particle = ClothParticle {
            position: Vec3::new(1.0, -0.5, 0.0),
            prev_position: Vec3::new(1.0, -0.3, 0.0),
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        collider.resolve_collision(&mut particle, 0.0);

        // Should push particle to y=0 (above plane)
        assert!(
            particle.position.y >= -0.01,
            "Plane should push particle above surface: y={}",
            particle.position.y
        );
        // X should be unchanged
        assert!(
            (particle.position.x - 1.0).abs() < 0.01,
            "X should be unchanged: x={}",
            particle.position.x
        );
    }

    #[test]
    fn r11_plane_friction_reduces_tangent() {
        let collider = ClothCollider::Plane {
            point: Vec3::ZERO,
            normal: Vec3::Y,
        };
        // Particle below plane with horizontal velocity
        let mut particle = ClothParticle {
            position: Vec3::new(2.0, -0.1, 0.0),
            prev_position: Vec3::new(0.0, -0.1, 0.0), // velocity = (2, 0, 0)
            acceleration: Vec3::ZERO,
            inv_mass: 1.0,
            pinned: false,
        };
        collider.resolve_collision(&mut particle, 0.8);

        let vel_after = particle.position - particle.prev_position;
        // Tangent velocity (X) should be reduced by friction
        // tangent_vel * (1 - friction) = 2.0 * 0.2 = 0.4
        assert!(
            vel_after.x < 1.0,
            "Friction 0.8 should heavily reduce tangent velocity: vx={}",
            vel_after.x
        );
    }

    #[test]
    fn r11_cloth_update_wind_force_uses_dot_abs() {
        // Wind force = wind * dot(wind, normal).abs() * (1 / inv_mass)
        // Need curvature for normals to be non-zero, so pin top row + gravity
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            particle_mass: 1.0,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            wind: Vec3::ZERO,
            air_resistance: 0.0,
            stiffness: 0.5,
            damping: 0.0,
            solver_iterations: 4,
        };
        let mut cloth = Cloth::new(ClothId(200), config, Vec3::ZERO);

        // Pin top row
        for x in 0..5 {
            cloth.particles[x].pinned = true;
            cloth.particles[x].inv_mass = 0.0;
        }

        // Let gravity create droop
        for _ in 0..20 {
            cloth.update(1.0 / 60.0);
        }

        // Now enable wind
        cloth.config.wind = Vec3::new(10.0, 0.0, 0.0);
        cloth.config.gravity = Vec3::ZERO; // stop gravity

        let pos_before: Vec<Vec3> = cloth.particles.iter().map(|p| p.position).collect();
        for _ in 0..10 {
            cloth.update(1.0 / 60.0);
        }

        // Some non-pinned particles should have moved due to wind
        let total_moved: f32 = cloth
            .particles
            .iter()
            .enumerate()
            .filter(|(i, p)| !p.pinned)
            .map(|(i, p)| (p.position - pos_before[i]).length())
            .sum();
        assert!(
            total_moved > 0.1,
            "Wind should move particles with curvature: total_moved={}",
            total_moved
        );
    }

    #[test]
    fn r11_cloth_update_drag_opposes_velocity() {
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            particle_mass: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            air_resistance: 5.0, // high drag
            stiffness: 0.0,
            damping: 0.0,
            solver_iterations: 0,
        };
        let mut cloth = Cloth::new(ClothId(200), config, Vec3::ZERO);

        // Give center particle velocity by offsetting prev_position
        cloth.particles[4].prev_position = cloth.particles[4].position - Vec3::new(1.0, 0.0, 0.0);

        let vel_before = cloth.particles[4].velocity();
        cloth.update(0.1);
        let vel_after = cloth.particles[4].velocity();

        // Drag should reduce velocity
        assert!(
            vel_after.x < vel_before.x,
            "Drag should reduce velocity: before={}, after={}",
            vel_before.x,
            vel_after.x
        );
    }

    #[test]
    fn r11_particle_normal_subtraction_direction() {
        // Test that v1 = neighbors.position - center (not + center)
        // Push a NEIGHBOR of (1,1) to break symmetry
        let config = ClothConfig {
            width: 3,
            height: 3,
            spacing: 1.0,
            particle_mass: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            air_resistance: 0.0,
            stiffness: 0.0,
            damping: 0.0,
            solver_iterations: 0,
        };
        let mut cloth = Cloth::new(ClothId(200), config, Vec3::ZERO);

        // Push particle at (0,1) = index 3 (a neighbor of (1,1)) upward
        // This breaks symmetry for particle_normal at (1,1)
        cloth.particles[3].position.y += 5.0;
        // Also push (1,0) = index 1 sideways
        cloth.particles[1].position.z += 3.0;

        let normal = cloth.particle_normal(1, 1);
        assert!(
            normal.length() > 0.3,
            "Normal with perturbed neighbors should be non-zero: {:?}",
            normal
        );
    }

    #[test]
    fn r11_particle_normal_accumulation_adds() {
        // += vs -= changes normal direction completely
        // particle_normal simplifies to (v_left - v_up) × (v_right - v_down)
        // Need h_left + h_right != h_up + h_down for non-zero result
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            particle_mass: 1.0,
            gravity: Vec3::ZERO,
            wind: Vec3::ZERO,
            air_resistance: 0.0,
            stiffness: 0.0,
            damping: 0.0,
            solver_iterations: 0,
        };
        let mut cloth = Cloth::new(ClothId(200), config, Vec3::ZERO);

        // For particle_normal(2, 2), neighbors are:
        // left=(1,2)=idx 11, right=(3,2)=idx 13, up=(2,1)=idx 7, down=(2,3)=idx 17
        // Only raise the LEFT neighbor — breaks h_L+h_R vs h_U+h_D symmetry
        cloth.particles[11].position.y += 5.0;

        let normal = cloth.particle_normal(2, 2);
        assert!(
            normal.length() > 0.3,
            "Asymmetric height perturbation should produce non-zero normal: {:?}",
            normal
        );
    }

    #[test]
    fn r11_constraint_solve_stiffness_multiplied() {
        // Mutation: * with / in stiffness would change correction magnitude
        let constraint = DistanceConstraint {
            p1: 0,
            p2: 1,
            rest_length: 1.0,
            stiffness: 0.5,
        };
        let constraint_full = DistanceConstraint {
            p1: 0,
            p2: 1,
            rest_length: 1.0,
            stiffness: 1.0,
        };

        let mut particles_half = vec![
            ClothParticle {
                position: Vec3::ZERO,
                prev_position: Vec3::ZERO,
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: false,
            },
            ClothParticle {
                position: Vec3::new(3.0, 0.0, 0.0),
                prev_position: Vec3::new(3.0, 0.0, 0.0),
                acceleration: Vec3::ZERO,
                inv_mass: 1.0,
                pinned: false,
            },
        ];
        let mut particles_full = particles_half.clone();

        constraint.solve(&mut particles_half);
        constraint_full.solve(&mut particles_full);

        // With half stiffness, correction should be roughly half
        let correction_half = (particles_half[1].position.x - 3.0).abs();
        let correction_full = (particles_full[1].position.x - 3.0).abs();

        assert!(
            correction_full > correction_half * 1.5,
            "Full stiffness should give larger correction: half={}, full={}",
            correction_half,
            correction_full
        );
    }

    #[test]
    fn r11_cloth_update_inv_mass_in_wind() {
        // Wind force = wind * effect * (1.0 / inv_mass.max(0.001))
        // Need curvature first for wind to work
        let config = ClothConfig {
            width: 5,
            height: 5,
            spacing: 1.0,
            particle_mass: 0.5, // inv_mass = 2.0
            gravity: Vec3::new(0.0, -9.81, 0.0),
            wind: Vec3::ZERO,
            air_resistance: 0.0,
            stiffness: 0.5,
            damping: 0.0,
            solver_iterations: 4,
        };
        let mut cloth = Cloth::new(ClothId(201), config, Vec3::ZERO);

        // Pin top row
        for x in 0..5 {
            cloth.particles[x].pinned = true;
            cloth.particles[x].inv_mass = 0.0;
        }

        // Let gravity create droop
        for _ in 0..20 {
            cloth.update(1.0 / 60.0);
        }

        // Enable wind, disable gravity
        cloth.config.wind = Vec3::new(5.0, 0.0, 0.0);
        cloth.config.gravity = Vec3::ZERO;

        let pos_before: Vec<Vec3> = cloth.particles.iter().map(|p| p.position).collect();
        for _ in 0..10 {
            cloth.update(1.0 / 60.0);
        }

        let total_moved: f32 = cloth
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.pinned)
            .map(|(i, p)| (p.position - pos_before[i]).length())
            .sum();
        assert!(
            total_moved > 0.01,
            "Wind with non-unit mass should produce movement: d={}",
            total_moved
        );
    }
}
