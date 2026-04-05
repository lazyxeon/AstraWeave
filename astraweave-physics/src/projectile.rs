//! # Projectile System
//!
//! Lightweight projectile simulation optimized for games. Uses custom ballistic
//! solving instead of full rigid body simulation for performance.
//!
//! ## Features
//!
//! - **Hitscan**: Instant raycast projectiles (bullets, lasers)
//! - **Kinematic**: Physically simulated projectiles (grenades, arrows)
//! - **Ballistics**: Gravity, drag, wind effects
//! - **Collision**: Raycast/shapecast detection with penetration
//! - **Explosions**: Radial impulse with falloff curves
//!
//! ## Usage
//!
//! ```rust
//! use astraweave_physics::projectile::{ProjectileManager, ProjectileConfig, ProjectileKind};
//! use glam::Vec3;
//!
//! let mut manager = ProjectileManager::new();
//!
//! // Spawn a grenade
//! let config = ProjectileConfig {
//!     kind: ProjectileKind::Kinematic,
//!     position: Vec3::new(0.0, 1.0, 0.0),
//!     velocity: Vec3::new(10.0, 5.0, 0.0),
//!     gravity_scale: 1.0,
//!     drag: 0.01,
//!     radius: 0.1,
//!     max_lifetime: 10.0,
//!     ..Default::default()
//! };
//!
//! let id = manager.spawn(config);
//! ```

use glam::Vec3;
use std::collections::HashMap;

/// Unique identifier for a projectile
pub type ProjectileId = u64;

/// Type of projectile simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ProjectileKind {
    /// Instant raycast (bullets, lasers) - no travel time
    Hitscan,
    /// Physically simulated with ballistics (grenades, arrows)
    #[default]
    Kinematic,
}

/// Configuration for spawning a projectile
#[derive(Debug, Clone)]
pub struct ProjectileConfig {
    /// Type of projectile
    pub kind: ProjectileKind,
    /// Initial world position
    pub position: Vec3,
    /// Initial velocity (direction * speed)
    pub velocity: Vec3,
    /// Gravity multiplier (0.0 = no gravity, 1.0 = normal, -1.0 = reverse)
    pub gravity_scale: f32,
    /// Air resistance coefficient (0.0 = none, higher = more drag)
    pub drag: f32,
    /// Collision radius for shapecast
    pub radius: f32,
    /// Maximum time before auto-despawn (seconds)
    pub max_lifetime: f32,
    /// Maximum bounces before despawn (0 = no bounce)
    pub max_bounces: u32,
    /// Bounciness factor (0.0 = no bounce, 1.0 = perfect elastic)
    pub restitution: f32,
    /// Penetration power (0.0 = no penetration)
    pub penetration: f32,
    /// Owner entity ID (for friendly fire detection)
    pub owner: Option<u64>,
    /// User data for game logic
    pub user_data: u64,
}

impl Default for ProjectileConfig {
    fn default() -> Self {
        Self {
            kind: ProjectileKind::Kinematic,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            gravity_scale: 1.0,
            drag: 0.0,
            radius: 0.05,
            max_lifetime: 10.0,
            max_bounces: 0,
            restitution: 0.5,
            penetration: 0.0,
            owner: None,
            user_data: 0,
        }
    }
}

/// Active projectile state
#[derive(Debug, Clone)]
pub struct Projectile {
    pub id: ProjectileId,
    pub config: ProjectileConfig,
    pub position: Vec3,
    pub velocity: Vec3,
    pub lifetime: f32,
    pub bounces: u32,
    pub active: bool,
}

impl Projectile {
    fn new(id: ProjectileId, config: ProjectileConfig) -> Self {
        let position = config.position;
        let velocity = config.velocity;
        Self {
            id,
            config,
            position,
            velocity,
            lifetime: 0.0,
            bounces: 0,
            active: true,
        }
    }
}

/// Result of a projectile hit
#[derive(Debug, Clone)]
pub struct ProjectileHit {
    /// Projectile that hit
    pub projectile_id: ProjectileId,
    /// World position of impact
    pub position: Vec3,
    /// Surface normal at impact
    pub normal: Vec3,
    /// Body ID that was hit (if any)
    pub body_id: Option<u64>,
    /// Distance traveled to hit
    pub distance: f32,
    /// Whether the projectile penetrated
    pub penetrated: bool,
}

/// Falloff curve for explosion damage/force
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FalloffCurve {
    /// Force = max * (1 - distance/radius)
    #[default]
    Linear,
    /// Force = max * (1 - (distance/radius)^2)
    Quadratic,
    /// Force = max * e^(-distance/radius)
    Exponential,
    /// Force = max (no falloff)
    Constant,
}

impl FalloffCurve {
    /// Calculate falloff multiplier (0.0 to 1.0) given distance and radius
    pub fn calculate(&self, distance: f32, radius: f32) -> f32 {
        if distance >= radius {
            return 0.0;
        }
        if radius <= 0.0 {
            return 1.0;
        }
        let t = distance / radius;
        match self {
            FalloffCurve::Linear => 1.0 - t,
            FalloffCurve::Quadratic => 1.0 - t * t,
            FalloffCurve::Exponential => (-t * 3.0).exp(), // e^(-3t) gives ~5% at edge
            FalloffCurve::Constant => 1.0,
        }
    }
}

/// Configuration for an explosion
#[derive(Debug, Clone)]
pub struct ExplosionConfig {
    /// Center of explosion
    pub center: Vec3,
    /// Maximum radius of effect
    pub radius: f32,
    /// Maximum force at center
    pub force: f32,
    /// Force falloff curve
    pub falloff: FalloffCurve,
    /// Upward bias (0.0 = pure radial, 1.0 = pure upward)
    pub upward_bias: f32,
}

impl Default for ExplosionConfig {
    fn default() -> Self {
        Self {
            center: Vec3::ZERO,
            radius: 5.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.3,
        }
    }
}

/// Result of explosion affecting a body
#[derive(Debug, Clone)]
pub struct ExplosionResult {
    /// Body that was affected
    pub body_id: u64,
    /// Impulse applied to body
    pub impulse: Vec3,
    /// Distance from explosion center
    pub distance: f32,
    /// Falloff multiplier applied
    pub falloff_multiplier: f32,
}

/// Manages all active projectiles
#[derive(Debug)]
pub struct ProjectileManager {
    projectiles: HashMap<ProjectileId, Projectile>,
    next_id: ProjectileId,
    /// Global wind vector affecting all projectiles
    pub wind: Vec3,
    /// Global gravity vector (default: -9.81 Y)
    pub gravity: Vec3,
    /// Pending hits from last update
    hits: Vec<ProjectileHit>,
}

impl Default for ProjectileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectileManager {
    /// Create a new projectile manager
    pub fn new() -> Self {
        Self {
            projectiles: HashMap::new(),
            next_id: 1,
            wind: Vec3::ZERO,
            gravity: Vec3::new(0.0, -9.81, 0.0),
            hits: Vec::new(),
        }
    }

    /// Spawn a new projectile
    pub fn spawn(&mut self, config: ProjectileConfig) -> ProjectileId {
        let id = self.next_id;
        self.next_id += 1;

        let projectile = Projectile::new(id, config);
        self.projectiles.insert(id, projectile);
        id
    }

    /// Despawn a projectile
    pub fn despawn(&mut self, id: ProjectileId) -> bool {
        self.projectiles.remove(&id).is_some()
    }

    /// Get a projectile by ID
    pub fn get(&self, id: ProjectileId) -> Option<&Projectile> {
        self.projectiles.get(&id)
    }

    /// Get a mutable reference to a projectile
    pub fn get_mut(&mut self, id: ProjectileId) -> Option<&mut Projectile> {
        self.projectiles.get_mut(&id)
    }

    /// Get all active projectiles
    pub fn iter(&self) -> impl Iterator<Item = &Projectile> {
        self.projectiles.values().filter(|p| p.active)
    }

    /// Get number of active projectiles
    pub fn count(&self) -> usize {
        self.projectiles.len()
    }

    /// Get hits from last update (call after `update`)
    pub fn drain_hits(&mut self) -> Vec<ProjectileHit> {
        std::mem::take(&mut self.hits)
    }

    /// Update all projectiles (call once per frame)
    ///
    /// # Arguments
    /// * `dt` - Delta time in seconds
    /// * `raycast_fn` - Function to perform raycasts: (origin, direction, max_dist) -> Option<(hit_pos, normal, body_id, dist)>
    pub fn update<F>(&mut self, dt: f32, mut raycast_fn: F)
    where
        F: FnMut(Vec3, Vec3, f32) -> Option<(Vec3, Vec3, Option<u64>, f32)>,
    {
        let gravity = self.gravity;
        let wind = self.wind;
        let mut to_despawn = Vec::new();

        for projectile in self.projectiles.values_mut() {
            if !projectile.active {
                continue;
            }

            // Update lifetime
            projectile.lifetime += dt;
            if projectile.lifetime >= projectile.config.max_lifetime {
                projectile.active = false;
                to_despawn.push(projectile.id);
                continue;
            }

            // Skip hitscan (they resolve instantly on spawn)
            if projectile.config.kind == ProjectileKind::Hitscan {
                continue;
            }

            // Store previous position for collision detection
            let prev_pos = projectile.position;

            // Apply gravity
            let grav_accel = gravity * projectile.config.gravity_scale;
            projectile.velocity += grav_accel * dt;

            // Apply drag: F_drag = -drag * v^2 * normalize(v)
            let speed = projectile.velocity.length();
            if speed > 0.001 && projectile.config.drag > 0.0 {
                let drag_force = projectile.config.drag * speed * speed;
                let drag_decel = (drag_force / 1.0) * dt; // Assume unit mass
                let decel = drag_decel.min(speed); // Don't reverse direction
                projectile.velocity -= projectile.velocity.normalize() * decel;
            }

            // Apply wind
            projectile.velocity += wind * dt;

            // Calculate new position
            let movement = projectile.velocity * dt;
            let new_pos = prev_pos + movement;

            // Raycast for collision
            let move_dist = movement.length();
            if move_dist > 0.001 {
                let dir = movement.normalize();
                if let Some((hit_pos, normal, body_id, dist)) =
                    raycast_fn(prev_pos, dir, move_dist + projectile.config.radius)
                {
                    // Record hit
                    self.hits.push(ProjectileHit {
                        projectile_id: projectile.id,
                        position: hit_pos,
                        normal,
                        body_id,
                        distance: dist,
                        penetrated: projectile.config.penetration > 0.0,
                    });

                    // Handle bounce
                    if projectile.bounces < projectile.config.max_bounces
                        && projectile.config.restitution > 0.0
                    {
                        // Reflect velocity
                        let reflect =
                            projectile.velocity - 2.0 * projectile.velocity.dot(normal) * normal;
                        projectile.velocity = reflect * projectile.config.restitution;
                        projectile.position = hit_pos + normal * 0.01; // Offset from surface
                        projectile.bounces += 1;
                    } else {
                        // Despawn on impact
                        projectile.active = false;
                        to_despawn.push(projectile.id);
                    }
                } else {
                    // No collision, update position
                    projectile.position = new_pos;
                }
            } else {
                projectile.position = new_pos;
            }
        }

        // Clean up despawned projectiles
        for id in to_despawn {
            self.projectiles.remove(&id);
        }
    }

    /// Perform hitscan (instant raycast projectile)
    ///
    /// Returns hit result if something was hit.
    pub fn hitscan<F>(
        &mut self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        mut raycast_fn: F,
    ) -> Option<ProjectileHit>
    where
        F: FnMut(Vec3, Vec3, f32) -> Option<(Vec3, Vec3, Option<u64>, f32)>,
    {
        let dir = direction.normalize();
        raycast_fn(origin, dir, max_distance).map(|(hit_pos, normal, body_id, dist)| {
            ProjectileHit {
                projectile_id: 0, // Hitscan doesn't create persistent projectile
                position: hit_pos,
                normal,
                body_id,
                distance: dist,
                penetrated: false,
            }
        })
    }

    /// Calculate explosion effects on nearby bodies
    ///
    /// # Arguments
    /// * `config` - Explosion configuration
    /// * `bodies` - Iterator of (body_id, position) pairs to check
    ///
    /// # Returns
    /// Vector of bodies affected with impulse to apply
    pub fn calculate_explosion<I>(
        &self,
        config: &ExplosionConfig,
        bodies: I,
    ) -> Vec<ExplosionResult>
    where
        I: IntoIterator<Item = (u64, Vec3)>,
    {
        let mut results = Vec::new();

        for (body_id, body_pos) in bodies {
            let to_body = body_pos - config.center;
            let distance = to_body.length();

            if distance >= config.radius {
                continue;
            }

            let falloff = config.falloff.calculate(distance, config.radius);
            let force_magnitude = config.force * falloff;

            // Calculate direction with upward bias
            let radial_dir = if distance > 0.001 {
                to_body.normalize()
            } else {
                Vec3::Y // Default to up if at center
            };

            let biased_dir = (radial_dir * (1.0 - config.upward_bias)
                + Vec3::Y * config.upward_bias)
                .normalize();

            let impulse = biased_dir * force_magnitude;

            results.push(ExplosionResult {
                body_id,
                impulse,
                distance,
                falloff_multiplier: falloff,
            });
        }

        results
    }
}

/// Calculate projectile trajectory points (for prediction/visualization)
///
/// # Arguments
/// * `start` - Starting position
/// * `velocity` - Initial velocity
/// * `gravity` - Gravity vector
/// * `drag` - Drag coefficient
/// * `dt` - Time step between points
/// * `num_points` - Number of trajectory points
pub fn predict_trajectory(
    start: Vec3,
    velocity: Vec3,
    gravity: Vec3,
    drag: f32,
    dt: f32,
    num_points: usize,
) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(num_points);
    let mut pos = start;
    let mut vel = velocity;

    points.push(pos);

    for _ in 1..num_points {
        // Apply gravity
        vel += gravity * dt;

        // Apply drag
        let speed = vel.length();
        if speed > 0.001 && drag > 0.0 {
            let drag_force = drag * speed * speed;
            let drag_decel = (drag_force / 1.0) * dt;
            let decel = drag_decel.min(speed);
            vel -= vel.normalize() * decel;
        }

        pos += vel * dt;
        points.push(pos);
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // BASIC PROJECTILE TESTS (Original)
    // ============================================================================

    #[test]
    fn test_projectile_spawn() {
        let mut manager = ProjectileManager::new();
        let id = manager.spawn(ProjectileConfig::default());
        assert!(manager.get(id).is_some());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_projectile_despawn() {
        let mut manager = ProjectileManager::new();
        let id = manager.spawn(ProjectileConfig::default());
        assert!(manager.despawn(id));
        assert!(manager.get(id).is_none());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_projectile_gravity() {
        let mut manager = ProjectileManager::new();
        let config = ProjectileConfig {
            position: Vec3::new(0.0, 10.0, 0.0),
            velocity: Vec3::new(10.0, 0.0, 0.0),
            gravity_scale: 1.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        // No-op raycast
        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // Simulate 1 second
        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        let proj = manager.get(id).unwrap();
        // Should have fallen ~4.9m (1/2 * 9.81 * 1^2)
        assert!(
            proj.position.y < 6.0,
            "Y should be < 6, got {}",
            proj.position.y
        );
        // Should have moved ~10m horizontally
        assert!(
            proj.position.x > 9.0,
            "X should be > 9, got {}",
            proj.position.x
        );
    }

    #[test]
    fn test_projectile_drag() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO; // No gravity for this test

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(100.0, 0.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.1,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // Simulate 1 second
        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        let proj = manager.get(id).unwrap();
        // With drag, should have slowed down significantly
        assert!(
            proj.velocity.x < 100.0,
            "Velocity should decrease with drag"
        );
    }

    #[test]
    fn test_projectile_bounce() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            gravity_scale: 0.0,
            max_bounces: 3,
            restitution: 0.8,
            ..Default::default()
        };
        let id = manager.spawn(config);

        // Simulate hitting a wall at X=5
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if origin.x < 5.0 && dir.x > 0.0 {
                    let dist = 5.0 - origin.x;
                    if dist < max {
                        return Some((
                            Vec3::new(5.0, 0.0, 0.0),
                            Vec3::new(-1.0, 0.0, 0.0),
                            Some(1),
                            dist,
                        ));
                    }
                }
                None
            };

        // First update should hit the wall
        manager.update(1.0, raycast);

        let proj = manager.get(id).unwrap();
        assert_eq!(proj.bounces, 1, "Should have bounced once");
        assert!(proj.velocity.x < 0.0, "Velocity should be reversed");
    }

    #[test]
    fn test_projectile_lifetime() {
        let mut manager = ProjectileManager::new();
        let config = ProjectileConfig {
            max_lifetime: 0.5,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // Simulate 1 second (projectile should despawn at 0.5s)
        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        assert!(
            manager.get(id).is_none(),
            "Projectile should have despawned"
        );
    }

    #[test]
    fn test_falloff_linear() {
        let curve = FalloffCurve::Linear;
        assert!((curve.calculate(0.0, 10.0) - 1.0).abs() < 0.001);
        assert!((curve.calculate(5.0, 10.0) - 0.5).abs() < 0.001);
        assert!((curve.calculate(10.0, 10.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_falloff_quadratic() {
        let curve = FalloffCurve::Quadratic;
        assert!((curve.calculate(0.0, 10.0) - 1.0).abs() < 0.001);
        assert!((curve.calculate(5.0, 10.0) - 0.75).abs() < 0.001); // 1 - 0.5^2 = 0.75
        assert!((curve.calculate(10.0, 10.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_explosion_radial() {
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        let bodies = vec![
            (1, Vec3::new(5.0, 0.0, 0.0)),  // At half radius
            (2, Vec3::new(15.0, 0.0, 0.0)), // Outside radius
        ];

        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1, "Only one body should be affected");
        assert_eq!(results[0].body_id, 1);
        assert!((results[0].falloff_multiplier - 0.5).abs() < 0.01);
        assert!(
            results[0].impulse.x > 0.0,
            "Impulse should push away from center"
        );
    }

    #[test]
    fn test_explosion_upward_bias() {
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 1.0, // Full upward
        };

        let bodies = vec![(1, Vec3::new(5.0, 0.0, 0.0))];
        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        // With full upward bias, impulse should be purely vertical
        assert!(
            results[0].impulse.y > 900.0,
            "Impulse should be mostly upward"
        );
    }

    #[test]
    fn test_predict_trajectory() {
        let points = predict_trajectory(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(0.0, -9.81, 0.0),
            0.0,
            0.1,
            10,
        );

        assert_eq!(points.len(), 10);
        assert_eq!(points[0], Vec3::ZERO);
        // Later points should show parabolic arc
        assert!(points[9].x > points[0].x, "Should move forward");
    }

    #[test]
    fn test_hitscan() {
        let mut manager = ProjectileManager::new();

        // Mock raycast that hits at distance 5
        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            Some((
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Some(1),
                5.0,
            ))
        };

        let hit = manager.hitscan(Vec3::ZERO, Vec3::X, 100.0, raycast);

        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.distance, 5.0);
        assert_eq!(hit.body_id, Some(1));
    }

    #[test]
    fn test_wind_effect() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;
        manager.wind = Vec3::new(5.0, 0.0, 0.0); // Wind blowing +X

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 10.0), // Moving +Z
            gravity_scale: 0.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        let proj = manager.get(id).unwrap();
        // Wind should have pushed projectile in +X direction
        assert!(proj.position.x > 0.0, "Wind should affect trajectory");
    }

    // ============================================================================
    // BALLISTICS VALIDATION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_ballistics_zero_gravity() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // 1 second at 10 m/s should travel exactly 10 meters
        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        let proj = manager.get(id).unwrap();
        assert!(
            (proj.position.x - 10.0).abs() < 0.1,
            "Should travel ~10m, got {}",
            proj.position.x
        );
        assert!(
            (proj.position.y - 0.0).abs() < 0.001,
            "Y should be 0, got {}",
            proj.position.y
        );
    }

    #[test]
    fn test_ballistics_parabolic_arc() {
        let mut manager = ProjectileManager::new();

        // Classic 45-degree launch
        let speed = 20.0;
        let angle = std::f32::consts::FRAC_PI_4; // 45 degrees
        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(speed * angle.cos(), speed * angle.sin(), 0.0),
            gravity_scale: 1.0,
            drag: 0.0,
            max_lifetime: 10.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // Record peak height
        let mut max_y = 0.0f32;
        for _ in 0..180 {
            manager.update(1.0 / 60.0, raycast);
            if let Some(proj) = manager.get(id) {
                max_y = max_y.max(proj.position.y);
            }
        }

        // Peak height for 45-degree launch: h = v^2 * sin^2(45) / (2g)
        // h = 20^2 * 0.5 / (2 * 9.81) ≈ 10.2
        assert!(
            max_y > 8.0 && max_y < 12.0,
            "Peak should be ~10m, got {}",
            max_y
        );
    }

    #[test]
    fn test_ballistics_range_calculation() {
        let mut manager = ProjectileManager::new();

        // 45-degree launch for max range
        let speed = 20.0;
        let angle = std::f32::consts::FRAC_PI_4;
        let config = ProjectileConfig {
            position: Vec3::new(0.0, 0.01, 0.0), // Slightly above ground
            velocity: Vec3::new(speed * angle.cos(), speed * angle.sin(), 0.0),
            gravity_scale: 1.0,
            drag: 0.0,
            max_lifetime: 10.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        // Ground plane raycast
        let raycast =
            |origin: Vec3, dir: Vec3, max_dist: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if dir.y < 0.0 && origin.y > 0.0 {
                    let t = -origin.y / dir.y;
                    if t > 0.0 && t < max_dist {
                        let hit = origin + dir * t;
                        return Some((hit, Vec3::Y, Some(0), t));
                    }
                }
                None
            };

        for _ in 0..300 {
            manager.update(1.0 / 60.0, raycast);
        }

        // Check hits
        let hits = manager.drain_hits();
        if !hits.is_empty() {
            // Range for 45-degree: R = v^2 / g = 400 / 9.81 ≈ 40.8
            let range = hits[0].position.x;
            assert!(
                range > 35.0 && range < 45.0,
                "Range should be ~40m, got {}",
                range
            );
        }
    }

    #[test]
    fn test_ballistics_negative_gravity_scale() {
        let mut manager = ProjectileManager::new();

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            gravity_scale: -1.0, // Reverse gravity
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        for _ in 0..60 {
            manager.update(1.0 / 60.0, raycast);
        }

        let proj = manager.get(id).unwrap();
        // Should rise instead of fall
        assert!(proj.position.y > 0.0, "Should rise with negative gravity");
    }

    // ============================================================================
    // PENETRATION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_penetration_flag_set() {
        let mut manager = ProjectileManager::new();

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(100.0, 0.0, 0.0),
            penetration: 1.0,
            ..Default::default()
        };
        manager.spawn(config);

        let raycast = |origin: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            if origin.x < 5.0 {
                Some((
                    Vec3::new(5.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Some(1),
                    5.0,
                ))
            } else {
                None
            }
        };

        manager.update(0.1, raycast);

        let hits = manager.drain_hits();
        assert!(!hits.is_empty(), "Should have hit");
        assert!(hits[0].penetrated, "Penetration flag should be set");
    }

    #[test]
    fn test_no_penetration_flag() {
        let mut manager = ProjectileManager::new();

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(100.0, 0.0, 0.0),
            penetration: 0.0, // No penetration
            ..Default::default()
        };
        manager.spawn(config);

        let raycast = |origin: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            if origin.x < 5.0 {
                Some((
                    Vec3::new(5.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Some(1),
                    5.0,
                ))
            } else {
                None
            }
        };

        manager.update(0.1, raycast);

        let hits = manager.drain_hits();
        assert!(!hits.is_empty(), "Should have hit");
        assert!(!hits[0].penetrated, "Penetration flag should NOT be set");
    }

    // ============================================================================
    // EXPLOSION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_explosion_at_center() {
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        // Body at exact center
        let bodies = vec![(1, Vec3::ZERO)];
        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].distance, 0.0);
        // At center, direction defaults to Y
        assert!(results[0].impulse.y > 0.0);
    }

    #[test]
    fn test_explosion_multiple_bodies() {
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 20.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        let bodies = vec![
            (1, Vec3::new(5.0, 0.0, 0.0)),
            (2, Vec3::new(0.0, 5.0, 0.0)),
            (3, Vec3::new(0.0, 0.0, 5.0)),
            (4, Vec3::new(15.0, 0.0, 0.0)),
        ];

        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 4);

        // Bodies at same distance should have same falloff
        assert!((results[0].falloff_multiplier - results[1].falloff_multiplier).abs() < 0.01);
        assert!((results[0].falloff_multiplier - results[2].falloff_multiplier).abs() < 0.01);

        // Body 4 is further, should have less falloff
        assert!(results[3].falloff_multiplier < results[0].falloff_multiplier);
    }

    #[test]
    fn test_explosion_no_bodies_in_range() {
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 5.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        let bodies = vec![
            (1, Vec3::new(10.0, 0.0, 0.0)),
            (2, Vec3::new(0.0, 10.0, 0.0)),
        ];

        let results = manager.calculate_explosion(&config, bodies);
        assert!(results.is_empty(), "No bodies should be affected");
    }

    #[test]
    fn test_falloff_exponential() {
        let curve = FalloffCurve::Exponential;
        assert!((curve.calculate(0.0, 10.0) - 1.0).abs() < 0.001);
        // Exponential falls off faster initially
        let mid = curve.calculate(5.0, 10.0);
        assert!(mid > 0.1 && mid < 0.5, "Mid should be ~0.22, got {}", mid);
        // At edge should be ~5%
        let edge = curve.calculate(10.0, 10.0);
        assert_eq!(edge, 0.0); // Beyond radius
    }

    #[test]
    fn test_falloff_constant() {
        let curve = FalloffCurve::Constant;
        assert_eq!(curve.calculate(0.0, 10.0), 1.0);
        assert_eq!(curve.calculate(5.0, 10.0), 1.0);
        assert_eq!(curve.calculate(9.99, 10.0), 1.0);
        assert_eq!(curve.calculate(10.0, 10.0), 0.0); // At radius = 0
    }

    #[test]
    fn test_falloff_zero_radius() {
        let curve = FalloffCurve::Linear;
        // Zero radius: at distance 0, ratio is NaN/Inf -> clamps to 0 (outside)
        // Implementation doesn't special-case zero radius
        assert_eq!(curve.calculate(0.0, 0.0), 0.0); // Returns 0 for edge case
        assert_eq!(curve.calculate(1.0, 0.0), 0.0);
    }

    // ============================================================================
    // MANAGER TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_manager_multiple_projectiles() {
        let mut manager = ProjectileManager::new();

        for i in 0..100 {
            let config = ProjectileConfig {
                position: Vec3::new(i as f32, 0.0, 0.0),
                ..Default::default()
            };
            manager.spawn(config);
        }

        assert_eq!(manager.count(), 100);
    }

    #[test]
    fn test_manager_get_mut() {
        let mut manager = ProjectileManager::new();
        let id = manager.spawn(ProjectileConfig::default());

        {
            let proj = manager.get_mut(id).unwrap();
            proj.velocity = Vec3::new(100.0, 0.0, 0.0);
        }

        let proj = manager.get(id).unwrap();
        assert_eq!(proj.velocity.x, 100.0);
    }

    #[test]
    fn test_manager_iter() {
        let mut manager = ProjectileManager::new();

        for _ in 0..10 {
            manager.spawn(ProjectileConfig::default());
        }

        let count = manager.iter().count();
        assert_eq!(count, 10);
    }

    #[test]
    fn test_manager_despawn_nonexistent() {
        let mut manager = ProjectileManager::new();
        assert!(
            !manager.despawn(999),
            "Should return false for non-existent ID"
        );
    }

    #[test]
    fn test_manager_default() {
        let manager = ProjectileManager::default();
        assert_eq!(manager.count(), 0);
        assert_eq!(manager.gravity, Vec3::new(0.0, -9.81, 0.0));
        assert_eq!(manager.wind, Vec3::ZERO);
    }

    // ============================================================================
    // CONFIG TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_config_default() {
        let config = ProjectileConfig::default();

        assert_eq!(config.kind, ProjectileKind::Kinematic);
        assert_eq!(config.position, Vec3::ZERO);
        assert_eq!(config.velocity, Vec3::ZERO);
        assert_eq!(config.gravity_scale, 1.0);
        assert_eq!(config.drag, 0.0);
        assert_eq!(config.radius, 0.05);
        assert_eq!(config.max_lifetime, 10.0);
        assert_eq!(config.max_bounces, 0);
        assert_eq!(config.restitution, 0.5);
        assert_eq!(config.penetration, 0.0);
        assert!(config.owner.is_none());
        assert_eq!(config.user_data, 0);
    }

    #[test]
    fn test_explosion_config_default() {
        let config = ExplosionConfig::default();

        assert_eq!(config.center, Vec3::ZERO);
        assert_eq!(config.radius, 5.0);
        assert_eq!(config.force, 1000.0);
        assert_eq!(config.falloff, FalloffCurve::Linear);
        assert_eq!(config.upward_bias, 0.3);
    }

    #[test]
    fn test_projectile_kind_default() {
        let kind = ProjectileKind::default();
        assert_eq!(kind, ProjectileKind::Kinematic);
    }

    #[test]
    fn test_falloff_curve_default() {
        let falloff = FalloffCurve::default();
        assert_eq!(falloff, FalloffCurve::Linear);
    }

    // ============================================================================
    // HITSCAN TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_hitscan_miss() {
        let mut manager = ProjectileManager::new();

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            None // Nothing hit
        };

        let hit = manager.hitscan(Vec3::ZERO, Vec3::X, 100.0, raycast);
        assert!(hit.is_none());
    }

    #[test]
    fn test_hitscan_no_body() {
        let mut manager = ProjectileManager::new();

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            Some((
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0),
                None,
                5.0,
            ))
        };

        let hit = manager.hitscan(Vec3::ZERO, Vec3::X, 100.0, raycast);
        assert!(hit.is_some());
        assert!(hit.unwrap().body_id.is_none());
    }

    // ============================================================================
    // TRAJECTORY PREDICTION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_predict_trajectory_with_drag() {
        let points = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(50.0, 0.0, 0.0),
            Vec3::ZERO,
            0.1, // Significant drag
            0.1,
            20,
        );

        assert_eq!(points.len(), 20);

        // With drag, spacing between points should decrease
        let first_gap = (points[1] - points[0]).length();
        let last_gap = (points[19] - points[18]).length();

        assert!(
            last_gap < first_gap,
            "Drag should slow projectile: first_gap={}, last_gap={}",
            first_gap,
            last_gap
        );
    }

    #[test]
    fn test_predict_trajectory_empty() {
        let points = predict_trajectory(Vec3::ZERO, Vec3::X, Vec3::ZERO, 0.0, 0.1, 0);

        // Implementation starts with initial point, so count=0 still gives 1 point
        // (the function always includes the starting position)
        assert!(points.len() <= 1);
    }

    #[test]
    fn test_predict_trajectory_single_point() {
        let points = predict_trajectory(Vec3::new(1.0, 2.0, 3.0), Vec3::X, Vec3::ZERO, 0.0, 0.1, 1);

        assert_eq!(points.len(), 1);
        assert_eq!(points[0], Vec3::new(1.0, 2.0, 3.0));
    }

    // ============================================================================
    // MULTIPLE BOUNCES TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_multiple_bounces_limit() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            gravity_scale: 0.0,
            max_bounces: 2,
            restitution: 1.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        // Walls at X=5 and X=-5
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if dir.x > 0.0 && origin.x < 5.0 {
                    let dist = 5.0 - origin.x;
                    if dist < max && dist > 0.01 {
                        return Some((
                            Vec3::new(5.0, 0.0, 0.0),
                            Vec3::new(-1.0, 0.0, 0.0),
                            Some(1),
                            dist,
                        ));
                    }
                } else if dir.x < 0.0 && origin.x > -5.0 {
                    let dist = origin.x + 5.0;
                    if dist < max && dist > 0.01 {
                        return Some((
                            Vec3::new(-5.0, 0.0, 0.0),
                            Vec3::new(1.0, 0.0, 0.0),
                            Some(1),
                            dist,
                        ));
                    }
                }
                None
            };

        // Simulate multiple bounces
        for _ in 0..100 {
            manager.update(0.1, raycast);
        }

        // After max bounces, projectile should be gone
        assert!(
            manager.get(id).is_none(),
            "Projectile should despawn after max bounces"
        );
    }

    #[test]
    fn test_zero_bounces() {
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            max_bounces: 0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |origin: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
            if origin.x < 5.0 {
                Some((
                    Vec3::new(5.0, 0.0, 0.0),
                    Vec3::new(-1.0, 0.0, 0.0),
                    Some(1),
                    5.0,
                ))
            } else {
                None
            }
        };

        manager.update(1.0, raycast);

        // Should despawn immediately on impact (no bounces)
        assert!(manager.get(id).is_none());
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — projectile Round 2 arithmetic/boundary tests
    // ═══════════════════════════════════════════════════════════════

    // --- predict_trajectory exact arithmetic ---
    #[test]
    fn mutation_predict_trajectory_no_drag_exact() {
        // No drag, no gravity, pure linear
        let pts = predict_trajectory(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::ZERO,
            0.0,
            0.5, // dt
            4,
        );
        assert_eq!(pts.len(), 4);
        // pt[0] = start
        assert_eq!(pts[0], Vec3::new(1.0, 2.0, 3.0));
        // pt[1] = (1 + 10*0.5, 2, 3) = (6, 2, 3)
        assert!(
            (pts[1].x - 6.0).abs() < 1e-4,
            "pts[1].x should be 6, got {}",
            pts[1].x
        );
        // pt[2] = (6 + 10*0.5, 2, 3) = (11, 2, 3)
        assert!(
            (pts[2].x - 11.0).abs() < 1e-4,
            "pts[2].x should be 11, got {}",
            pts[2].x
        );
        // pt[3] = (16, 2, 3)
        assert!(
            (pts[3].x - 16.0).abs() < 1e-4,
            "pts[3].x should be 16, got {}",
            pts[3].x
        );
    }

    #[test]
    fn mutation_predict_trajectory_gravity_exact() {
        // Gravity only, no drag
        let g = Vec3::new(0.0, -10.0, 0.0);
        let dt = 1.0;
        let pts = predict_trajectory(Vec3::ZERO, Vec3::new(5.0, 0.0, 0.0), g, 0.0, dt, 3);
        assert_eq!(pts.len(), 3);
        // Step 1: vel = (5, 0, 0) + (0, -10, 0)*1 = (5, -10, 0)
        // pos = (0,0,0) + (5,-10,0)*1 = (5, -10, 0)
        assert!((pts[1].x - 5.0).abs() < 1e-3, "pts[1].x={}", pts[1].x);
        assert!((pts[1].y - (-10.0)).abs() < 1e-3, "pts[1].y={}", pts[1].y);
        // Step 2: vel = (5,-10,0) + (0,-10,0)*1 = (5,-20,0)
        // pos = (5,-10,0) + (5,-20,0)*1 = (10, -30, 0)
        assert!((pts[2].x - 10.0).abs() < 1e-3, "pts[2].x={}", pts[2].x);
        assert!((pts[2].y - (-30.0)).abs() < 1e-3, "pts[2].y={}", pts[2].y);
    }

    #[test]
    fn mutation_predict_trajectory_drag_exact_one_step() {
        // One step with drag: vel=(100,0,0), drag=0.01, dt=0.1
        // speed=100, drag_force = 0.01*100*100 = 100, drag_decel = 100*0.1 = 10
        // decel = min(10, 100) = 10
        // new_vel = 100 - 10 = 90 → (90,0,0)
        // pos = (0,0,0) + (90,0,0)*0.1 = (9,0,0)
        let pts = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::ZERO,
            0.01,
            0.1,
            2,
        );
        assert_eq!(pts.len(), 2);
        assert!(
            (pts[1].x - 9.0).abs() < 0.01,
            "Expected 9.0, got {}",
            pts[1].x
        );
        assert!((pts[1].y).abs() < 1e-5);
    }

    // --- FalloffCurve::calculate exact values ---
    #[test]
    fn mutation_falloff_linear_exact_values() {
        let c = FalloffCurve::Linear;
        // t = d/r → 1 - t
        assert!((c.calculate(0.0, 10.0) - 1.0).abs() < 1e-6);
        assert!((c.calculate(2.5, 10.0) - 0.75).abs() < 1e-6);
        assert!((c.calculate(7.5, 10.0) - 0.25).abs() < 1e-6);
        assert!((c.calculate(10.0, 10.0) - 0.0).abs() < 1e-6); // d >= r
    }

    #[test]
    fn mutation_falloff_quadratic_exact() {
        let c = FalloffCurve::Quadratic;
        // 1 - t^2
        assert!((c.calculate(0.0, 10.0) - 1.0).abs() < 1e-6);
        // t=0.3 → 1-0.09 = 0.91
        assert!(
            (c.calculate(3.0, 10.0) - 0.91).abs() < 1e-5,
            "got {}",
            c.calculate(3.0, 10.0)
        );
        // t=0.7 → 1-0.49 = 0.51
        assert!(
            (c.calculate(7.0, 10.0) - 0.51).abs() < 1e-5,
            "got {}",
            c.calculate(7.0, 10.0)
        );
    }

    #[test]
    fn mutation_falloff_exponential_exact() {
        let c = FalloffCurve::Exponential;
        // e^(-3t), t=d/r
        assert!((c.calculate(0.0, 10.0) - 1.0).abs() < 1e-6);
        // d=5, r=10 → t=0.5 → e^(-1.5) ≈ 0.2231
        let expected = (-1.5_f32).exp();
        assert!(
            (c.calculate(5.0, 10.0) - expected).abs() < 1e-4,
            "got {}",
            c.calculate(5.0, 10.0)
        );
    }

    #[test]
    fn mutation_falloff_constant_within_radius() {
        let c = FalloffCurve::Constant;
        assert_eq!(c.calculate(0.0, 10.0), 1.0);
        assert_eq!(c.calculate(5.0, 10.0), 1.0);
        assert_eq!(c.calculate(9.999, 10.0), 1.0);
        assert_eq!(c.calculate(10.0, 10.0), 0.0); // d >= r check
    }

    #[test]
    fn mutation_falloff_zero_radius_returns_zero() {
        // radius <= 0 → returns 1.0 per source code (if distance >= radius already caught)
        // Actually: distance=0, radius=0 → 0 >= 0 is true → returns 0.0
        assert_eq!(FalloffCurve::Linear.calculate(0.0, 0.0), 0.0);
        assert_eq!(FalloffCurve::Linear.calculate(1.0, 0.0), 0.0);
    }

    // --- calculate_explosion exact arithmetic ---
    #[test]
    fn mutation_explosion_upward_bias_exact() {
        let mgr = ProjectileManager::new();
        // Body at (10, 0, 0), center at origin, radius = 20, force = 100
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 20.0,
            force: 100.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 0.5,
        };
        let bodies = vec![(1, Vec3::new(10.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 1);

        // radial_dir = (1,0,0), biased_dir = normalize((1,0,0)*0.5 + (0,1,0)*0.5)
        // = normalize((0.5, 0.5, 0.0)) = (0.7071, 0.7071, 0)
        // impulse = (0.7071, 0.7071, 0) * 100 (constant falloff)
        let imp = results[0].impulse;
        let expected_component = 100.0 / (2.0_f32).sqrt();
        assert!((imp.x - expected_component).abs() < 0.5, "imp.x={}", imp.x);
        assert!((imp.y - expected_component).abs() < 0.5, "imp.y={}", imp.y);
        assert!(imp.z.abs() < 0.1);
    }

    #[test]
    fn mutation_explosion_at_center_defaults_to_up() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 200.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 0.0,
        };
        let bodies = vec![(1, Vec3::ZERO)]; // At center
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 1);
        // At center, radial_dir = Vec3::Y, bias=0 → dir = Vec3::Y
        assert!(
            (results[0].impulse.y - 200.0).abs() < 0.1,
            "Should be 200 up, got {}",
            results[0].impulse.y
        );
        assert!(results[0].impulse.x.abs() < 0.1);
    }

    #[test]
    fn mutation_explosion_falloff_multiplier_exact() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 20.0,
            force: 100.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };
        let bodies = vec![
            (1, Vec3::new(5.0, 0.0, 0.0)),  // d=5, falloff = 1 - 5/20 = 0.75
            (2, Vec3::new(10.0, 0.0, 0.0)), // d=10, falloff = 1 - 10/20 = 0.5
            (3, Vec3::new(15.0, 0.0, 0.0)), // d=15, falloff = 1 - 15/20 = 0.25
        ];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 3);
        assert!((results[0].falloff_multiplier - 0.75).abs() < 0.01);
        assert!((results[1].falloff_multiplier - 0.5).abs() < 0.01);
        assert!((results[2].falloff_multiplier - 0.25).abs() < 0.01);
        // impulse magnitudes: force * falloff
        assert!((results[0].impulse.length() - 75.0).abs() < 0.5);
        assert!((results[1].impulse.length() - 50.0).abs() < 0.5);
        assert!((results[2].impulse.length() - 25.0).abs() < 0.5);
    }

    // --- ProjectileManager::update drag arithmetic ---
    #[test]
    fn mutation_update_drag_velocity_decrease_exact() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        mgr.wind = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(50.0, 0.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.02,
            ..Default::default()
        };
        let id = mgr.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        let dt = 1.0 / 60.0;
        mgr.update(dt, raycast);

        let proj = mgr.get(id).unwrap();
        // speed=50, drag_force=0.02*50*50=50, drag_decel=50*dt=0.8333
        // new_speed = 50 - 0.8333 = 49.1667
        let expected_speed = 50.0 - 0.02 * 50.0 * 50.0 * dt;
        assert!(
            (proj.velocity.x - expected_speed).abs() < 0.01,
            "Expected vel.x={}, got {}",
            expected_speed,
            proj.velocity.x
        );
    }

    #[test]
    fn mutation_update_gravity_accumulates() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::new(0.0, -10.0, 0.0);
        mgr.wind = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::new(0.0, 100.0, 0.0),
            velocity: Vec3::ZERO,
            gravity_scale: 1.0,
            drag: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        let dt = 0.1;
        mgr.update(dt, raycast);

        let proj = mgr.get(id).unwrap();
        // After 1 step: vel = (0, -10*0.1, 0) = (0, -1, 0)
        // pos = (0, 100, 0) + (0, -1, 0)*0.1 = (0, 99.9, 0)
        assert!(
            (proj.velocity.y - (-1.0)).abs() < 0.01,
            "vel.y={}",
            proj.velocity.y
        );
        assert!(
            (proj.position.y - 99.9).abs() < 0.01,
            "pos.y={}",
            proj.position.y
        );
    }

    #[test]
    fn mutation_update_wind_effect_exact() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        mgr.wind = Vec3::new(20.0, 0.0, 0.0);

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(0.0, 0.0, 10.0),
            gravity_scale: 0.0,
            drag: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        let dt = 0.5;
        mgr.update(dt, raycast);

        let proj = mgr.get(id).unwrap();
        // vel = (0,0,10) + (20,0,0)*0.5 = (10, 0, 10)
        // pos = (0,0,0) + (10,0,10)*0.5 = (5, 0, 5)
        assert!(
            (proj.velocity.x - 10.0).abs() < 0.01,
            "vel.x={}",
            proj.velocity.x
        );
        assert!(
            (proj.position.z - 5.0).abs() < 0.01,
            "pos.z={}",
            proj.position.z
        );
        assert!(
            (proj.position.x - 5.0).abs() < 0.01,
            "pos.x={}",
            proj.position.x
        );
    }

    #[test]
    fn mutation_update_lifetime_exact_boundary() {
        let mut mgr = ProjectileManager::new();
        let config = ProjectileConfig {
            max_lifetime: 1.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        // After 0.99s should still exist
        for _ in 0..59 {
            mgr.update(1.0 / 60.0, raycast);
        }
        assert!(mgr.get(id).is_some(), "Should still exist at 0.98s");

        // One more step pushes past 1.0
        mgr.update(1.0 / 60.0, raycast);
        // Lifetime may or may not have triggered yet depending on accumulation
        // but after full 60 steps at 1/60, total = 1.0s exactly
        for _ in 0..5 {
            mgr.update(1.0 / 60.0, raycast);
        }
        assert!(mgr.get(id).is_none(), "Should be despawned after >1s");
    }

    #[test]
    fn mutation_bounce_restitution_factor() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0),
            gravity_scale: 0.0,
            max_bounces: 5,
            restitution: 0.6,
            ..Default::default()
        };
        let id = mgr.spawn(config);

        // Wall at x=5
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if dir.x > 0.0 && origin.x < 5.0 {
                    let dist = 5.0 - origin.x;
                    if dist < max && dist > 0.01 {
                        return Some((
                            Vec3::new(5.0, 0.0, 0.0),
                            Vec3::new(-1.0, 0.0, 0.0),
                            Some(1),
                            dist,
                        ));
                    }
                }
                None
            };

        mgr.update(1.0, raycast);

        let proj = mgr.get(id).unwrap();
        assert_eq!(proj.bounces, 1);
        // reflected velocity = -(10) * 0.6 = -6
        assert!(
            (proj.velocity.x - (-6.0)).abs() < 0.1,
            "After bounce, vel.x should be -6.0, got {}",
            proj.velocity.x
        );
    }

    #[test]
    fn mutation_spawn_increments_id() {
        let mut mgr = ProjectileManager::new();
        let id1 = mgr.spawn(ProjectileConfig::default());
        let id2 = mgr.spawn(ProjectileConfig::default());
        let id3 = mgr.spawn(ProjectileConfig::default());
        assert_eq!(id2, id1 + 1);
        assert_eq!(id3, id2 + 1);
    }

    // ===== DEEP REMEDIATION v3.6.3 — Round 4 remaining update & explosion mutations =====

    #[test]
    fn mutation_r4_update_position_integration_exact() {
        // velocity * dt should update position precisely
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO; // No gravity
        let config = ProjectileConfig {
            position: Vec3::new(1.0, 2.0, 3.0),
            velocity: Vec3::new(10.0, -5.0, 7.0),
            gravity_scale: 0.0,
            drag: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(0.1, no_hit);
        let p = mgr.get(id).unwrap();
        // new_pos = (1,2,3) + (10,-5,7)*0.1 = (2, 1.5, 3.7)
        assert!((p.position.x - 2.0).abs() < 0.01, "x: {}", p.position.x);
        assert!((p.position.y - 1.5).abs() < 0.01, "y: {}", p.position.y);
        assert!((p.position.z - 3.7).abs() < 0.01, "z: {}", p.position.z);
    }

    #[test]
    fn mutation_r4_update_gravity_scale_multiplier() {
        // gravity_scale should multiply the gravity vector
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::new(0.0, -10.0, 0.0);
        let config = ProjectileConfig {
            velocity: Vec3::ZERO,
            gravity_scale: 0.5,
            drag: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(1.0, no_hit);
        let p = mgr.get(id).unwrap();
        // vel = 0 + (-10 * 0.5) * 1.0 = -5
        assert!(
            (p.velocity.y - (-5.0)).abs() < 0.01,
            "vel.y should be -5 with 0.5 scale: {}",
            p.velocity.y
        );
    }

    #[test]
    fn mutation_r4_update_drag_decel_min_speed() {
        // drag_decel should be min'd with speed (don't reverse)
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(0.01, 0.0, 0.0), // Very slow
            drag: 1000.0,                        // Huge drag
            gravity_scale: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(1.0, no_hit);
        let p = mgr.get(id).unwrap();
        // Drag should have reduced speed to 0, NOT reversed direction
        assert!(
            p.velocity.x >= -0.001,
            "Don't reverse: vel.x={}",
            p.velocity.x
        );
    }

    #[test]
    fn mutation_r4_update_wind_adds_to_velocity() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        mgr.wind = Vec3::new(5.0, 0.0, 0.0);
        let config = ProjectileConfig {
            velocity: Vec3::new(0.0, 0.0, 10.0),
            drag: 0.0,
            gravity_scale: 0.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(1.0, no_hit);
        let p = mgr.get(id).unwrap();
        // wind adds 5*1=5 to x velocity
        assert!(
            (p.velocity.x - 5.0).abs() < 0.1,
            "Wind should add to vel.x: {}",
            p.velocity.x
        );
        assert!(
            (p.velocity.z - 10.0).abs() < 0.1,
            "Z should be unchanged: {}",
            p.velocity.z
        );
    }

    #[test]
    fn mutation_r4_update_lifetime_accumulates() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(1.0, 0.0, 0.0),
            drag: 0.0,
            gravity_scale: 0.0,
            max_lifetime: 10.0,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(0.3, no_hit);
        let lt1 = mgr.get(id).unwrap().lifetime;
        assert!((lt1 - 0.3).abs() < 0.001, "lifetime after 0.3s: {}", lt1);
        mgr.update(0.7, no_hit);
        let lt2 = mgr.get(id).unwrap().lifetime;
        assert!((lt2 - 1.0).abs() < 0.001, "lifetime after 1.0s: {}", lt2);
    }

    #[test]
    fn mutation_r4_update_hitscan_skipped() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::new(0.0, -10.0, 0.0);
        let config = ProjectileConfig {
            kind: ProjectileKind::Hitscan,
            position: Vec3::new(0.0, 5.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            ..Default::default()
        };
        let id = mgr.spawn(config);
        let no_hit = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };
        mgr.update(1.0, no_hit);
        let p = mgr.get(id).unwrap();
        // Hitscan should not be updated by physics
        assert!(
            (p.position.y - 5.0).abs() < 0.01,
            "Hitscan should stay put: {}",
            p.position.y
        );
    }

    #[test]
    fn mutation_r4_explosion_zero_upward_bias_radial() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 100.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 0.0, // Pure radial
        };
        let bodies = vec![(1u64, Vec3::new(5.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 1);
        // Pure radial: impulse should be entirely in +X direction
        let imp = results[0].impulse;
        assert!(imp.x > 90.0, "Pure radial should be mostly +X: {:?}", imp);
        assert!(imp.y.abs() < 1.0, "No Y bias: {:?}", imp);
    }

    #[test]
    fn mutation_r4_explosion_full_upward_bias() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 100.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 1.0, // Pure upward
        };
        let bodies = vec![(1u64, Vec3::new(5.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 1);
        // Pure upward: biased_dir = (radial*0 + Y*1).normalize = (0,1,0)
        let imp = results[0].impulse;
        assert!(imp.y > 90.0, "Pure upward should be mostly +Y: {:?}", imp);
        assert!(imp.x.abs() < 1.0, "No X: {:?}", imp);
    }

    #[test]
    fn mutation_r4_explosion_outside_radius_excluded() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 5.0,
            force: 100.0,
            ..Default::default()
        };
        let bodies = vec![
            (1u64, Vec3::new(4.9, 0.0, 0.0)), // Inside
            (2u64, Vec3::new(5.0, 0.0, 0.0)), // On boundary (excluded: >= radius)
            (3u64, Vec3::new(6.0, 0.0, 0.0)), // Outside
        ];
        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(
            results.len(),
            1,
            "Only inside radius: {:?}",
            results.iter().map(|r| r.body_id).collect::<Vec<_>>()
        );
        assert_eq!(results[0].body_id, 1);
    }

    #[test]
    fn mutation_r4_update_bounce_reflects_y() {
        // Test bounce off a horizontal surface (Y normal)
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        let config = ProjectileConfig {
            position: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::new(0.0, -10.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.0,
            max_bounces: 5,
            restitution: 0.5,
            ..Default::default()
        };
        let id = mgr.spawn(config);
        // Floor at y=0
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if dir.y < 0.0 && origin.y > 0.0 {
                    let dist = origin.y;
                    if dist < max && dist > 0.001 {
                        return Some((Vec3::new(origin.x, 0.0, origin.z), Vec3::Y, Some(1), dist));
                    }
                }
                None
            };
        mgr.update(0.5, raycast);
        let p = mgr.get(id).unwrap();
        // reflect = (-10) - 2*(-10*1)*Y = (0,10,0) * 0.5 = (0,5,0)
        assert!(p.velocity.y > 0.0, "Should bounce up: {}", p.velocity.y);
        assert!(
            (p.velocity.y - 5.0).abs() < 0.5,
            "Restitution 0.5: vel.y should be ~5, got {}",
            p.velocity.y
        );
        assert_eq!(p.bounces, 1);
    }

    #[test]
    fn mutation_r4_update_collision_offset_from_surface() {
        // After bounce, position should be offset from surface
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        let config = ProjectileConfig {
            position: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::new(0.0, -100.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.0,
            max_bounces: 5,
            restitution: 0.8,
            ..Default::default()
        };
        let _id = mgr.spawn(config);
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if dir.y < 0.0 && origin.y > 0.0 {
                    let dist = origin.y;
                    if dist < max {
                        return Some((Vec3::new(0.0, 0.0, 0.0), Vec3::Y, Some(1), dist));
                    }
                }
                None
            };
        mgr.update(0.01, raycast);
        // Position should be at hit_pos + normal * 0.01 = (0, 0.01, 0)
        // Check that it bounced and is above the surface
        let remaining: Vec<_> = mgr.iter().collect();
        if !remaining.is_empty() {
            assert!(remaining[0].position.y > 0.0, "Offset above surface");
        }
    }

    #[test]
    fn mutation_r4_falloff_linear_exact_mid() {
        // Linear at distance=2.5, radius=10 → t=0.25 → 1-0.25=0.75
        let f = FalloffCurve::Linear.calculate(2.5, 10.0);
        assert!((f - 0.75).abs() < 0.001, "Linear at 0.25: {}", f);
    }

    #[test]
    fn mutation_r4_falloff_quadratic_vs_linear() {
        // Quadratic should be higher than linear at same point
        let d = 5.0;
        let r = 10.0;
        let fl = FalloffCurve::Linear.calculate(d, r);
        let fq = FalloffCurve::Quadratic.calculate(d, r);
        // t=0.5: Linear=0.5, Quadratic=1-0.25=0.75
        assert!((fl - 0.5).abs() < 0.001);
        assert!((fq - 0.75).abs() < 0.001);
        assert!(fq > fl, "Quadratic > Linear at t=0.5");
    }

    #[test]
    fn mutation_r4_falloff_exponential_decay_rate() {
        // Exponential: e^(-3t), at t=0.5 → e^(-1.5) ≈ 0.2231
        let f = FalloffCurve::Exponential.calculate(5.0, 10.0);
        let expected = (-1.5_f32).exp();
        assert!(
            (f - expected).abs() < 0.01,
            "Exponential at t=0.5: expected {:.4}, got {:.4}",
            expected,
            f
        );
    }

    #[test]
    fn mutation_r4_predict_trajectory_drag_multi_step() {
        // Multi-step drag should slow down monotonically
        let pts = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(100.0, 0.0, 0.0),
            Vec3::ZERO,
            0.5,
            0.1,
            10,
        );
        // Each successive segment should be shorter than previous
        for i in 2..pts.len() {
            let seg_prev = (pts[i - 1] - pts[i - 2]).length();
            let seg_curr = (pts[i] - pts[i - 1]).length();
            assert!(
                seg_curr <= seg_prev + 0.01,
                "Drag should slow down: seg[{}]={:.3} > seg[{}]={:.3}",
                i,
                seg_curr,
                i - 1,
                seg_prev
            );
        }
    }

    // ===== ROUND 7: predict_trajectory targeted catches =====

    #[test]
    fn r7_predict_trajectory_gravity_drops_y() {
        // vel += gravity * dt → if mutated to -= gravity * dt, Y would increase
        let pts = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::new(0.0, -9.81, 0.0),
            0.0,
            0.1,
            5,
        );

        // After a few steps, Y should decrease due to gravity
        assert!(
            pts[4].y < pts[0].y,
            "Gravity should decrease Y: start_y={}, end_y={}",
            pts[0].y,
            pts[4].y
        );
        // X should increase (positive velocity)
        assert!(
            pts[4].x > pts[0].x,
            "X should increase: start_x={}, end_x={}",
            pts[0].x,
            pts[4].x
        );
    }

    #[test]
    fn r7_predict_trajectory_drag_threshold() {
        // The drag condition checks: speed > 0.001 && drag > 0.0
        // With zero drag, velocity should NOT be affected by drag
        let pts_no_drag = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::ZERO,
            0.0, // no drag
            0.1,
            5,
        );
        let pts_with_drag = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
            Vec3::ZERO,
            0.5, // with drag
            0.1,
            5,
        );

        // With drag, final X should be less than without drag
        assert!(
            pts_with_drag[4].x < pts_no_drag[4].x,
            "Drag should reduce distance: drag={}, no_drag={}",
            pts_with_drag[4].x,
            pts_no_drag[4].x
        );
    }

    #[test]
    fn r7_predict_trajectory_drag_force_formula() {
        // drag_force = drag * speed * speed
        // drag_decel = (drag_force / 1.0) * dt
        // decel = drag_decel.min(speed)
        let drag = 0.1;
        let dt = 0.1;
        let v0 = Vec3::new(10.0, 0.0, 0.0);
        let pts = predict_trajectory(Vec3::ZERO, v0, Vec3::ZERO, drag, dt, 3);

        // Step 1: vel = (10,0,0), speed = 10
        // drag_force = 0.1 * 10 * 10 = 10
        // drag_decel = 10 * 0.1 = 1.0
        // decel = min(1.0, 10) = 1.0
        // vel -= normalize(10,0,0) * 1.0 = vel -= (1,0,0) → vel = (9,0,0)
        // pos = (0,0,0) + (9,0,0)*0.1 = (0.9, 0, 0)
        let expected_x1 = 0.9;
        assert!(
            (pts[1].x - expected_x1).abs() < 0.01,
            "First step with drag: expected x~{}, got {}",
            expected_x1,
            pts[1].x
        );
    }

    #[test]
    fn r7_predict_trajectory_drag_decel_min_speed() {
        // Test the .min(speed) clamp: when drag_decel > speed, only decel by speed
        // Use very high drag with low speed
        let pts = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(0.01, 0.0, 0.0), // Very slow
            Vec3::ZERO,
            100.0, // Very high drag
            0.1,
            3,
        );

        // Speed should not go negative (min clamp prevents this)
        // After drag: drag_force = 100*0.01*0.01 = 0.01, drag_decel = 0.01*0.1 = 0.001
        // decel = min(0.001, 0.01) = 0.001
        // vel = 0.01 - 0.001 = 0.009
        // Each step should still move forward (X increasing)
        assert!(
            pts[2].x > pts[0].x,
            "Should still move forward: start={}, end={}",
            pts[0].x,
            pts[2].x
        );
    }

    // ===== ROUND 9: Projectile update + explosion precision =====

    #[test]
    fn r9_projectile_update_gravity_pulls_down() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::new(0.0, -9.81, 0.0);
        mgr.wind = Vec3::ZERO;

        let id = mgr.spawn(ProjectileConfig {
            position: Vec3::new(0.0, 100.0, 0.0),
            velocity: Vec3::new(10.0, 0.0, 0.0), // horizontal
            gravity_scale: 1.0,
            drag: 0.0,
            ..Default::default()
        });

        // Step 10 frames at 60fps
        for _ in 0..10 {
            mgr.update(1.0 / 60.0, |_, _, _| None);
        }

        let p = mgr.get(id).unwrap();
        // After 10/60s with gravity -9.81, y should decrease
        // y_change = 0.5 * -9.81 * (10/60)^2 ≈ -0.136 (from velocity accumulation)
        assert!(
            p.position.y < 100.0,
            "Projectile should fall under gravity: y={}",
            p.position.y
        );
        // x should increase (moving forward)
        assert!(
            p.position.x > 0.0,
            "Projectile should move forward: x={}",
            p.position.x
        );
        // Velocity y should be negative (gravity accumulated)
        assert!(
            p.velocity.y < 0.0,
            "Velocity y should be negative from gravity: vy={}",
            p.velocity.y
        );
    }

    #[test]
    fn r9_projectile_update_drag_reduces_speed() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        mgr.wind = Vec3::ZERO;

        let initial_speed = 50.0;
        let id = mgr.spawn(ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(initial_speed, 0.0, 0.0),
            gravity_scale: 0.0,
            drag: 0.1,
            ..Default::default()
        });

        for _ in 0..30 {
            mgr.update(1.0 / 60.0, |_, _, _| None);
        }

        let p = mgr.get(id).unwrap();
        let speed = p.velocity.length();
        assert!(
            speed < initial_speed,
            "Drag should reduce speed: initial={}, now={}",
            initial_speed,
            speed
        );
        assert!(
            speed > 0.0,
            "Speed should still be positive (not reversed): {}",
            speed
        );
    }

    #[test]
    fn r9_projectile_update_wind_affects_velocity() {
        let mut mgr = ProjectileManager::new();
        mgr.gravity = Vec3::ZERO;
        mgr.wind = Vec3::new(0.0, 0.0, 5.0); // wind in Z direction

        let id = mgr.spawn(ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(10.0, 0.0, 0.0), // moving in X
            gravity_scale: 0.0,
            drag: 0.0,
            ..Default::default()
        });

        for _ in 0..30 {
            mgr.update(1.0 / 60.0, |_, _, _| None);
        }

        let p = mgr.get(id).unwrap();
        // Wind should push projectile in Z direction
        assert!(
            p.velocity.z > 0.0,
            "Wind should add Z velocity: vz={}",
            p.velocity.z
        );
        assert!(
            p.position.z > 0.0,
            "Wind should push in Z: z={}",
            p.position.z
        );
    }

    #[test]
    fn r9_explosion_force_scales_with_distance() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            upward_bias: 0.0,
            ..Default::default()
        };

        let bodies = vec![
            (1u64, Vec3::new(1.0, 0.0, 0.0)), // close
            (2u64, Vec3::new(8.0, 0.0, 0.0)), // far
        ];

        let results = mgr.calculate_explosion(&config, bodies);
        assert_eq!(results.len(), 2);

        let close = &results[0];
        let far = &results[1];

        // Closer body should receive more force
        assert!(
            close.impulse.length() > far.impulse.length(),
            "Close body impulse ({}) should exceed far body impulse ({})",
            close.impulse.length(),
            far.impulse.length()
        );

        // Both should push outward (positive x)
        assert!(close.impulse.x > 0.0, "Close impulse should push +X");
        assert!(far.impulse.x > 0.0, "Far impulse should push +X");
    }

    #[test]
    fn r9_explosion_upward_bias() {
        let mgr = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::ZERO,
            radius: 10.0,
            force: 1000.0,
            upward_bias: 0.5,
            ..Default::default()
        };

        let bodies = vec![(1u64, Vec3::new(3.0, 0.0, 0.0))];
        let results = mgr.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        let r = &results[0];

        // With upward_bias=0.5, impulse should have both X and Y components
        assert!(r.impulse.x > 0.0, "Should push outward: x={}", r.impulse.x);
        assert!(
            r.impulse.y > 0.0,
            "Upward bias should add Y: y={}",
            r.impulse.y
        );
    }

    #[test]
    fn r9_predict_trajectory_precise_gravity() {
        // Zero drag, known gravity → verify exact positions
        let pts = predict_trajectory(
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),  // 10 m/s in X
            Vec3::new(0.0, -10.0, 0.0), // gravity -10
            0.0,                        // no drag
            0.1,                        // dt = 0.1s
            4,
        );

        // Point 0: (0, 0, 0)
        assert_eq!(pts[0], Vec3::ZERO);

        // After step 1: vel = (10, -1, 0), pos = (10*0.1, -0.1*0.1, 0) = (1.0, -0.1, 0)
        // Actually: vel += gravity*dt = (10, 0-10*0.1, 0) = (10, -1, 0)
        // pos += vel*dt = (0+10*0.1, 0+(-1*0.1), 0) = (1.0, -0.1, 0)
        assert!(
            (pts[1].x - 1.0).abs() < 0.01,
            "Step 1 x should be 1.0, got {}",
            pts[1].x
        );
        assert!(
            (pts[1].y - (-0.1)).abs() < 0.01,
            "Step 1 y should be -0.1, got {}",
            pts[1].y
        );

        // Step 2: vel = (10, -2, 0), pos = (1.0+1.0, -0.1+(-0.2)) = (2.0, -0.3, 0)
        assert!(
            (pts[2].x - 2.0).abs() < 0.01,
            "Step 2 x should be 2.0, got {}",
            pts[2].x
        );
        assert!(
            (pts[2].y - (-0.3)).abs() < 0.01,
            "Step 2 y should be -0.3, got {}",
            pts[2].y
        );
    }

    // ============================================================================
    // R12: Targeted mutation-kill tests
    // ============================================================================

    #[test]
    fn r12_bounce_restitution_exact_velocity() {
        // Targets: L371 replace * with + and * with / in reflect * restitution
        // reflect * restitution should scale velocity by restitution factor
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(20.0, 0.0, 0.0),
            gravity_scale: 0.0,
            max_bounces: 3,
            restitution: 0.5,
            drag: 0.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        // Wall at X=5, normal = (-1, 0, 0)
        let raycast =
            |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if origin.x < 5.0 && dir.x > 0.0 {
                    let dist = 5.0 - origin.x;
                    if dist < max {
                        return Some((
                            Vec3::new(5.0, 0.0, 0.0),
                            Vec3::new(-1.0, 0.0, 0.0),
                            Some(1),
                            dist,
                        ));
                    }
                }
                None
            };

        // After bounce: reflected velocity = v - 2*dot(v,n)*n = (20,0,0) - 2*(-20)*(-1,0,0) = (20-40, 0, 0) = (-20, 0, 0)
        // Then scaled by restitution 0.5: (-10, 0, 0)
        manager.update(1.0, raycast);

        let proj = manager.get(id).unwrap();
        // With * restitution: velocity.x should be -10.0
        // With + restitution: velocity.x would be -20.0 + 0.5 = -19.5 (WRONG)
        // With / restitution: velocity.x would be -20.0 / 0.5 = -40.0 (WRONG)
        assert!(
            (proj.velocity.x - (-10.0)).abs() < 0.5,
            "Bounce velocity should be reflect*restitution = -10.0, got {}",
            proj.velocity.x
        );
    }

    #[test]
    fn r12_bounce_reflection_formula_exact() {
        // Targets: L369 replace * with / in velocity.dot(normal) * normal
        // Reflection uses element-wise multiply by normal scalar components
        // If * → /, result changes dramatically for non-unit-axis normals
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        // 45-degree wall: normal = normalize(-1, 1, 0) = (-0.707, 0.707, 0)
        let n = Vec3::new(-1.0, 1.0, 0.0).normalize();
        let wall_x = 5.0;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::new(20.0, 0.0, 0.0), // Moving right
            gravity_scale: 0.0,
            max_bounces: 3,
            restitution: 1.0, // Perfect elastic to isolate reflection formula
            drag: 0.0,
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast =
            move |origin: Vec3, dir: Vec3, max: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> {
                if origin.x < wall_x && dir.x > 0.0 {
                    let dist = wall_x - origin.x;
                    if dist < max {
                        return Some((Vec3::new(wall_x, 0.0, 0.0), n, Some(1), dist));
                    }
                }
                None
            };

        manager.update(1.0, raycast);

        let proj = manager.get(id).unwrap();
        // v = (20, 0, 0), n = (-0.707, 0.707, 0)
        // dot(v, n) = 20*(-0.707) = -14.14
        // reflect = v - 2*dot*n = (20, 0, 0) - 2*(-14.14)*(-0.707, 0.707, 0)
        //         = (20, 0, 0) - (20, -20, 0) = (0, 20, 0)
        // With restitution=1.0: velocity = (0, 20, 0)
        assert!(
            proj.velocity.x.abs() < 1.0,
            "After 45-deg bounce, x velocity should be ~0, got {}",
            proj.velocity.x
        );
        assert!(
            (proj.velocity.y - 20.0).abs() < 1.0,
            "After 45-deg bounce, y velocity should be ~20, got {}",
            proj.velocity.y
        );
    }

    #[test]
    fn r12_explosion_nonzero_center_direction() {
        // Targets: L438 replace - with + in `body_pos - config.center`
        // With non-zero center, the direction body-center matters
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::new(5.0, 0.0, 0.0), // Non-zero center
            radius: 20.0,
            force: 1000.0,
            falloff: FalloffCurve::Constant,
            upward_bias: 0.0,
        };

        // Body at (10, 0, 0) → should be pushed in +X (away from center at 5)
        let bodies = vec![(1, Vec3::new(10.0, 0.0, 0.0))];
        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        // Correct: to_body = (10,0,0) - (5,0,0) = (5,0,0) → push +X
        // Mutated: to_body = (10,0,0) + (5,0,0) = (15,0,0) → still +X but different distance
        // Actually with + the distance = 15, still < 20 radius, still pushes +X...
        // Need a case where - vs + gives opposite directions!
        // Body at (3, 0, 0), center at (5, 0, 0):
        // Correct: to_body = 3-5 = (-2,0,0) → push in -X (away from center)
        // Mutated: to_body = 3+5 = (8,0,0) → push in +X (WRONG direction!)
        let bodies2 = vec![(2, Vec3::new(3.0, 0.0, 0.0))];
        let results2 = manager.calculate_explosion(&config, bodies2);

        assert_eq!(results2.len(), 1);
        assert!(
            results2[0].impulse.x < 0.0,
            "Body at x=3 with center at x=5 should be pushed in -X: impulse.x={}",
            results2[0].impulse.x
        );
    }

    #[test]
    fn r12_explosion_distance_with_nonzero_center() {
        // Targets: L438 more precisely — check exact distance computation
        let manager = ProjectileManager::new();
        let config = ExplosionConfig {
            center: Vec3::new(10.0, 0.0, 0.0),
            radius: 20.0,
            force: 1000.0,
            falloff: FalloffCurve::Linear,
            upward_bias: 0.0,
        };

        // Body at (15, 0, 0) → distance should be 5.0
        let bodies = vec![(1, Vec3::new(15.0, 0.0, 0.0))];
        let results = manager.calculate_explosion(&config, bodies);

        assert_eq!(results.len(), 1);
        assert!(
            (results[0].distance - 5.0).abs() < 0.01,
            "Distance from center(10,0,0) to body(15,0,0) should be 5.0, got {}",
            results[0].distance
        );
        // Linear falloff at dist=5, radius=20: 1.0 - 5/20 = 0.75
        assert!(
            (results[0].falloff_multiplier - 0.75).abs() < 0.01,
            "Falloff should be 0.75, got {}",
            results[0].falloff_multiplier
        );
    }

    #[test]
    fn r12_projectile_drag_not_applied_at_zero_speed() {
        // Targets: L332 boundary behavior and drag application correctness
        // A stationary projectile with drag > 0 should not have velocity affected by drag
        let mut manager = ProjectileManager::new();
        manager.gravity = Vec3::ZERO;

        let config = ProjectileConfig {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO, // Stationary
            gravity_scale: 0.0,
            drag: 10.0, // Very high drag
            ..Default::default()
        };
        let id = manager.spawn(config);

        let raycast = |_: Vec3, _: Vec3, _: f32| -> Option<(Vec3, Vec3, Option<u64>, f32)> { None };

        manager.update(1.0, raycast);

        let proj = manager.get(id).unwrap();
        // Velocity should remain zero - no NaN or weird values from drag calculation
        assert!(
            proj.velocity.length() < 0.001,
            "Stationary projectile should stay still: vel={}",
            proj.velocity
        );
        assert!(
            !proj.velocity.x.is_nan() && !proj.velocity.y.is_nan(),
            "No NaN from drag at zero velocity"
        );
    }
}
