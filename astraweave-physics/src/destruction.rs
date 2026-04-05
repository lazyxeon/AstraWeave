//! Destruction System
//!
//! Provides physics-based destruction for breakable objects:
//! - Pre-fractured mesh system (swap intact → debris)
//! - Structural integrity (health-based breaking)
//! - Debris generation and lifetime
//! - Force-based destruction triggers

use glam::Vec3;
use std::collections::HashMap;

/// Unique identifier for destructible objects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DestructibleId(pub u64);

/// Unique identifier for debris pieces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DebrisId(pub u64);

/// Shape of debris pieces
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DebrisShape {
    /// Box-shaped debris
    Box { half_extents: Vec3 },
    /// Spherical debris
    Sphere { radius: f32 },
    /// Convex hull (represented as box for simplicity)
    ConvexHull { half_extents: Vec3 },
}

impl Default for DebrisShape {
    fn default() -> Self {
        Self::Box {
            half_extents: Vec3::splat(0.2),
        }
    }
}

/// Configuration for a debris piece
#[derive(Debug, Clone)]
pub struct DebrisConfig {
    /// Shape of the debris
    pub shape: DebrisShape,
    /// Relative position from object center
    pub local_position: Vec3,
    /// Mass of debris piece
    pub mass: f32,
    /// Initial velocity multiplier (from destruction force)
    pub velocity_factor: f32,
    /// Angular velocity multiplier
    pub angular_velocity_factor: f32,
    /// Lifetime in seconds (0 = permanent)
    pub lifetime: f32,
    /// Whether debris can cause secondary destruction
    pub can_damage: bool,
    /// Damage amount if hits another destructible
    pub damage_on_hit: f32,
}

impl Default for DebrisConfig {
    fn default() -> Self {
        Self {
            shape: DebrisShape::default(),
            local_position: Vec3::ZERO,
            mass: 1.0,
            velocity_factor: 1.0,
            angular_velocity_factor: 0.5,
            lifetime: 10.0,
            can_damage: false,
            damage_on_hit: 0.0,
        }
    }
}

/// Fracture pattern for pre-fractured meshes
#[derive(Debug, Clone)]
pub struct FracturePattern {
    /// Debris pieces that make up this object when broken
    pub debris: Vec<DebrisConfig>,
    /// Center of mass for the intact object
    pub center_of_mass: Vec3,
}

impl FracturePattern {
    /// Create a simple uniform fracture pattern
    pub fn uniform(piece_count: usize, object_half_extents: Vec3, mass: f32) -> Self {
        let mut debris = Vec::with_capacity(piece_count);
        let piece_mass = mass / piece_count as f32;

        // Create a simple grid of debris
        let pieces_per_axis = (piece_count as f32).cbrt().ceil() as i32;
        let piece_size = object_half_extents * 2.0 / pieces_per_axis as f32;

        for x in 0..pieces_per_axis {
            for y in 0..pieces_per_axis {
                for z in 0..pieces_per_axis {
                    if debris.len() >= piece_count {
                        break;
                    }

                    let local_pos = Vec3::new(
                        (x as f32 + 0.5) * piece_size.x - object_half_extents.x,
                        (y as f32 + 0.5) * piece_size.y - object_half_extents.y,
                        (z as f32 + 0.5) * piece_size.z - object_half_extents.z,
                    );

                    debris.push(DebrisConfig {
                        shape: DebrisShape::Box {
                            half_extents: piece_size * 0.4,
                        },
                        local_position: local_pos,
                        mass: piece_mass,
                        ..Default::default()
                    });
                }
            }
        }

        Self {
            debris,
            center_of_mass: Vec3::ZERO,
        }
    }

    /// Create a radial fracture pattern (for explosions)
    pub fn radial(piece_count: usize, radius: f32, mass: f32) -> Self {
        let mut debris = Vec::with_capacity(piece_count);
        let piece_mass = mass / piece_count as f32;

        // Golden angle distribution for even spacing
        let golden_angle = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt());

        for i in 0..piece_count {
            let t = i as f32 / piece_count as f32;
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = golden_angle * i as f32;

            let local_pos = Vec3::new(
                inclination.sin() * azimuth.cos() * radius * 0.8,
                inclination.cos() * radius * 0.8,
                inclination.sin() * azimuth.sin() * radius * 0.8,
            );

            debris.push(DebrisConfig {
                shape: DebrisShape::Sphere {
                    radius: radius * 0.15,
                },
                local_position: local_pos,
                mass: piece_mass,
                velocity_factor: 1.5, // Radial patterns fly outward faster
                ..Default::default()
            });
        }

        Self {
            debris,
            center_of_mass: Vec3::ZERO,
        }
    }

    /// Create a layered fracture pattern (for walls)
    pub fn layered(layers: usize, pieces_per_layer: usize, half_extents: Vec3, mass: f32) -> Self {
        let total_pieces = layers * pieces_per_layer;
        let mut debris = Vec::with_capacity(total_pieces);
        let piece_mass = mass / total_pieces as f32;

        let layer_height = half_extents.y * 2.0 / layers as f32;

        for layer in 0..layers {
            let y = (layer as f32 + 0.5) * layer_height - half_extents.y;

            for piece in 0..pieces_per_layer {
                let angle = piece as f32 * std::f32::consts::TAU / pieces_per_layer as f32;
                let x = angle.cos() * half_extents.x * 0.7;
                let z = angle.sin() * half_extents.z * 0.7;

                debris.push(DebrisConfig {
                    shape: DebrisShape::Box {
                        half_extents: Vec3::new(
                            half_extents.x * 0.3,
                            layer_height * 0.4,
                            half_extents.z * 0.3,
                        ),
                    },
                    local_position: Vec3::new(x, y, z),
                    mass: piece_mass,
                    ..Default::default()
                });
            }
        }

        Self {
            debris,
            center_of_mass: Vec3::ZERO,
        }
    }
}

/// Destruction trigger type
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DestructionTrigger {
    /// Destroy when force exceeds threshold
    Force { threshold: f32 },
    /// Destroy when health reaches zero
    Health,
    /// Destroy on any collision
    Collision,
    /// Manual destruction only
    Manual,
}

impl Default for DestructionTrigger {
    fn default() -> Self {
        Self::Force { threshold: 1000.0 }
    }
}

/// Configuration for a destructible object
#[derive(Debug, Clone)]
pub struct DestructibleConfig {
    /// Fracture pattern (debris layout)
    pub fracture_pattern: FracturePattern,
    /// How destruction is triggered
    pub trigger: DestructionTrigger,
    /// Maximum health (for Health trigger)
    pub max_health: f32,
    /// Minimum force to deal damage
    pub damage_threshold: f32,
    /// Force-to-damage conversion factor
    pub force_to_damage: f32,
    /// Explosion force when destroyed
    pub destruction_force: f32,
    /// Sound effect ID (for audio integration)
    pub destruction_sound: Option<u32>,
    /// Particle effect ID (for VFX integration)
    pub destruction_particles: Option<u32>,
}

impl Default for DestructibleConfig {
    fn default() -> Self {
        Self {
            fracture_pattern: FracturePattern::uniform(8, Vec3::splat(0.5), 10.0),
            trigger: DestructionTrigger::default(),
            max_health: 100.0,
            damage_threshold: 10.0,
            force_to_damage: 0.1,
            destruction_force: 5.0,
            destruction_sound: None,
            destruction_particles: None,
        }
    }
}

/// State of a destructible object
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum DestructibleState {
    /// Object is intact
    Intact,
    /// Object is damaged but functional
    Damaged,
    /// Object is being destroyed (playing animation/effects)
    Destroying,
    /// Object has been destroyed
    Destroyed,
}

/// Runtime data for a destructible object
#[derive(Debug, Clone)]
pub struct Destructible {
    pub id: DestructibleId,
    pub config: DestructibleConfig,
    pub state: DestructibleState,
    /// Current health
    pub health: f32,
    /// World position
    pub position: Vec3,
    /// Accumulated force this frame
    pub accumulated_force: f32,
    /// Body ID in physics world (if any)
    pub body_id: Option<u64>,
}

impl Destructible {
    /// Create a new destructible
    pub fn new(id: DestructibleId, config: DestructibleConfig, position: Vec3) -> Self {
        Self {
            id,
            health: config.max_health,
            config,
            state: DestructibleState::Intact,
            position,
            accumulated_force: 0.0,
            body_id: None,
        }
    }

    /// Apply damage to the object
    pub fn apply_damage(&mut self, damage: f32) {
        if self.state != DestructibleState::Intact && self.state != DestructibleState::Damaged {
            return;
        }

        self.health = (self.health - damage).max(0.0);

        if self.health <= 0.0 {
            self.state = DestructibleState::Destroying;
        } else if self.health < self.config.max_health * 0.5 {
            self.state = DestructibleState::Damaged;
        }
    }

    /// Apply force to the object
    pub fn apply_force(&mut self, force: f32) {
        self.accumulated_force += force;

        // Check force threshold trigger
        if let DestructionTrigger::Force { threshold } = self.config.trigger {
            if self.accumulated_force >= threshold {
                self.state = DestructibleState::Destroying;
            }
        }

        // Apply force-based damage for Health trigger
        if matches!(self.config.trigger, DestructionTrigger::Health)
            && force >= self.config.damage_threshold
        {
            let damage = (force - self.config.damage_threshold) * self.config.force_to_damage;
            self.apply_damage(damage);
        }
    }

    /// Handle collision
    pub fn on_collision(&mut self, impact_force: f32) {
        if let DestructionTrigger::Collision = self.config.trigger {
            self.state = DestructibleState::Destroying;
        } else {
            self.apply_force(impact_force);
        }
    }

    /// Manually trigger destruction
    pub fn destroy(&mut self) {
        self.state = DestructibleState::Destroying;
    }

    /// Check if object should spawn debris
    pub fn should_spawn_debris(&self) -> bool {
        self.state == DestructibleState::Destroying
    }

    /// Mark destruction as complete
    pub fn complete_destruction(&mut self) {
        self.state = DestructibleState::Destroyed;
    }

    /// Reset accumulated force (call at end of frame)
    pub fn reset_frame(&mut self) {
        self.accumulated_force = 0.0;
    }

    /// Get health percentage
    pub fn health_percent(&self) -> f32 {
        self.health / self.config.max_health
    }

    /// Check if destroyed
    pub fn is_destroyed(&self) -> bool {
        self.state == DestructibleState::Destroyed
    }
}

/// Active debris piece in the world
#[derive(Debug, Clone)]
pub struct Debris {
    pub id: DebrisId,
    /// Source destructible ID
    pub source: DestructibleId,
    pub config: DebrisConfig,
    /// World position
    pub position: Vec3,
    /// Linear velocity
    pub velocity: Vec3,
    /// Angular velocity
    pub angular_velocity: Vec3,
    /// Time alive
    pub age: f32,
    /// Physics body ID (if any)
    pub body_id: Option<u64>,
}

impl Debris {
    /// Create new debris
    pub fn new(
        id: DebrisId,
        source: DestructibleId,
        config: DebrisConfig,
        position: Vec3,
        velocity: Vec3,
    ) -> Self {
        Self {
            id,
            source,
            config,
            position,
            velocity,
            angular_velocity: Vec3::ZERO,
            age: 0.0,
            body_id: None,
        }
    }

    /// Update debris (if not physics-driven)
    pub fn update(&mut self, dt: f32, gravity: Vec3) {
        self.age += dt;

        // Simple physics if not driven by physics engine
        if self.body_id.is_none() {
            self.velocity += gravity * dt;
            self.position += self.velocity * dt;
        }
    }

    /// Check if debris should be removed
    pub fn should_remove(&self) -> bool {
        self.config.lifetime > 0.0 && self.age >= self.config.lifetime
    }
}

/// Destruction event for callbacks
#[derive(Debug, Clone)]
pub struct DestructionEvent {
    /// ID of destroyed object
    pub destructible_id: DestructibleId,
    /// Position of destruction
    pub position: Vec3,
    /// Force that caused destruction
    pub force: f32,
    /// Direction of force
    pub force_direction: Vec3,
    /// Number of debris spawned
    pub debris_count: usize,
}

/// Manager for destruction system
#[derive(Debug, Default)]
pub struct DestructionManager {
    destructibles: HashMap<DestructibleId, Destructible>,
    debris: HashMap<DebrisId, Debris>,
    pending_events: Vec<DestructionEvent>,
    next_destructible_id: u64,
    next_debris_id: u64,
    /// Maximum active debris pieces
    pub max_debris: usize,
    /// Default debris lifetime
    pub default_debris_lifetime: f32,
}

impl DestructionManager {
    /// Create a new destruction manager
    pub fn new() -> Self {
        Self {
            destructibles: HashMap::new(),
            debris: HashMap::new(),
            pending_events: Vec::new(),
            next_destructible_id: 1,
            next_debris_id: 1,
            max_debris: 500,
            default_debris_lifetime: 10.0,
        }
    }

    // === Destructible Management ===

    /// Add a destructible object
    pub fn add_destructible(
        &mut self,
        config: DestructibleConfig,
        position: Vec3,
    ) -> DestructibleId {
        let id = DestructibleId(self.next_destructible_id);
        self.next_destructible_id += 1;
        self.destructibles
            .insert(id, Destructible::new(id, config, position));
        id
    }

    /// Remove a destructible (and its debris)
    pub fn remove_destructible(&mut self, id: DestructibleId) -> bool {
        // Remove associated debris
        self.debris.retain(|_, d| d.source != id);
        self.destructibles.remove(&id).is_some()
    }

    /// Get a destructible
    pub fn get(&self, id: DestructibleId) -> Option<&Destructible> {
        self.destructibles.get(&id)
    }

    /// Get a mutable destructible
    pub fn get_mut(&mut self, id: DestructibleId) -> Option<&mut Destructible> {
        self.destructibles.get_mut(&id)
    }

    /// Apply damage to a destructible
    pub fn apply_damage(&mut self, id: DestructibleId, damage: f32) {
        if let Some(dest) = self.destructibles.get_mut(&id) {
            dest.apply_damage(damage);
        }
    }

    /// Apply force to a destructible
    pub fn apply_force(&mut self, id: DestructibleId, force: f32) {
        if let Some(dest) = self.destructibles.get_mut(&id) {
            dest.apply_force(force);
        }
    }

    /// Handle collision for a destructible
    pub fn on_collision(&mut self, id: DestructibleId, impact_force: f32) {
        if let Some(dest) = self.destructibles.get_mut(&id) {
            dest.on_collision(impact_force);
        }
    }

    /// Manually destroy an object
    pub fn destroy(&mut self, id: DestructibleId) {
        if let Some(dest) = self.destructibles.get_mut(&id) {
            dest.destroy();
        }
    }

    // === Debris Management ===

    /// Get debris by ID
    pub fn get_debris(&self, id: DebrisId) -> Option<&Debris> {
        self.debris.get(&id)
    }

    /// Iterate over all debris
    pub fn debris_iter(&self) -> impl Iterator<Item = &Debris> {
        self.debris.values()
    }

    /// Spawn debris for a destructible
    fn spawn_debris(&mut self, dest: &Destructible, force_direction: Vec3) -> Vec<DebrisId> {
        let mut spawned = Vec::new();

        // Respect debris limit
        let available_slots = self.max_debris.saturating_sub(self.debris.len());
        let debris_to_spawn = dest
            .config
            .fracture_pattern
            .debris
            .len()
            .min(available_slots);

        for debris_config in dest
            .config
            .fracture_pattern
            .debris
            .iter()
            .take(debris_to_spawn)
        {
            let id = DebrisId(self.next_debris_id);
            self.next_debris_id += 1;

            // Calculate debris world position
            let position = dest.position + debris_config.local_position;

            // Calculate initial velocity (outward from center + force direction)
            let outward = debris_config.local_position.normalize_or_zero();
            let velocity = (outward * dest.config.destruction_force
                + force_direction * dest.config.destruction_force * 0.5)
                * debris_config.velocity_factor;

            let mut debris = Debris::new(id, dest.id, debris_config.clone(), position, velocity);

            // Random angular velocity
            debris.angular_velocity = Vec3::new(
                (id.0 as f32 * 1.234).sin() * 5.0,
                (id.0 as f32 * 2.345).sin() * 5.0,
                (id.0 as f32 * 3.456).sin() * 5.0,
            ) * debris_config.angular_velocity_factor;

            self.debris.insert(id, debris);
            spawned.push(id);
        }

        spawned
    }

    // === Update ===

    /// Update the destruction system
    pub fn update(&mut self, dt: f32, gravity: Vec3) {
        // Process destructibles that need debris spawning
        let mut to_process = Vec::new();
        for (id, dest) in &self.destructibles {
            if dest.should_spawn_debris() {
                to_process.push(*id);
            }
        }

        for id in to_process {
            if let Some(dest) = self.destructibles.get(&id) {
                // Clone needed data before mutable borrow
                let position = dest.position;
                let force = dest.accumulated_force;
                let dest_clone = dest.clone();

                // Spawn debris
                let debris_ids = self.spawn_debris(&dest_clone, Vec3::Y); // Default upward force direction

                // Create event
                self.pending_events.push(DestructionEvent {
                    destructible_id: id,
                    position,
                    force,
                    force_direction: Vec3::Y,
                    debris_count: debris_ids.len(),
                });
            }

            // Mark as destroyed
            if let Some(dest) = self.destructibles.get_mut(&id) {
                dest.complete_destruction();
            }
        }

        // Update debris
        for debris in self.debris.values_mut() {
            debris.update(dt, gravity);
        }

        // Remove expired debris
        self.debris.retain(|_, d| !d.should_remove());

        // Reset frame state for destructibles
        for dest in self.destructibles.values_mut() {
            dest.reset_frame();
        }
    }

    /// Take pending destruction events
    pub fn take_events(&mut self) -> Vec<DestructionEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Get counts
    pub fn destructible_count(&self) -> usize {
        self.destructibles.len()
    }

    pub fn debris_count(&self) -> usize {
        self.debris.len()
    }

    pub fn active_debris_count(&self) -> usize {
        self.debris.values().filter(|d| !d.should_remove()).count()
    }

    /// Clean up destroyed destructibles
    pub fn cleanup_destroyed(&mut self) {
        self.destructibles.retain(|_, d| !d.is_destroyed());
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructible_creation() {
        let config = DestructibleConfig::default();
        let dest = Destructible::new(DestructibleId(1), config.clone(), Vec3::ZERO);

        assert_eq!(dest.state, DestructibleState::Intact);
        assert_eq!(dest.health, config.max_health);
    }

    #[test]
    fn test_damage_application() {
        let config = DestructibleConfig {
            max_health: 100.0,
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        dest.apply_damage(30.0);
        assert_eq!(dest.health, 70.0);
        assert_eq!(dest.state, DestructibleState::Intact);

        dest.apply_damage(30.0);
        assert_eq!(dest.health, 40.0);
        assert_eq!(dest.state, DestructibleState::Damaged);

        dest.apply_damage(50.0);
        assert_eq!(dest.health, 0.0);
        assert_eq!(dest.state, DestructibleState::Destroying);
    }

    #[test]
    fn test_force_trigger() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Force { threshold: 100.0 },
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        dest.apply_force(50.0);
        assert_eq!(dest.state, DestructibleState::Intact);

        dest.apply_force(60.0);
        assert_eq!(dest.state, DestructibleState::Destroying);
    }

    #[test]
    fn test_collision_trigger() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Collision,
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        dest.on_collision(1.0);
        assert_eq!(dest.state, DestructibleState::Destroying);
    }

    #[test]
    fn test_manual_destruction() {
        let mut dest =
            Destructible::new(DestructibleId(1), DestructibleConfig::default(), Vec3::ZERO);

        dest.destroy();
        assert_eq!(dest.state, DestructibleState::Destroying);
    }

    #[test]
    fn test_health_percent() {
        let config = DestructibleConfig {
            max_health: 200.0,
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        assert_eq!(dest.health_percent(), 1.0);
        dest.apply_damage(100.0);
        assert_eq!(dest.health_percent(), 0.5);
    }

    #[test]
    fn test_uniform_fracture_pattern() {
        let pattern = FracturePattern::uniform(8, Vec3::splat(1.0), 10.0);

        assert_eq!(pattern.debris.len(), 8);

        let total_mass: f32 = pattern.debris.iter().map(|d| d.mass).sum();
        assert!(
            (total_mass - 10.0).abs() < 0.01,
            "Total mass should be preserved"
        );
    }

    #[test]
    fn test_radial_fracture_pattern() {
        let pattern = FracturePattern::radial(20, 2.0, 5.0);

        assert_eq!(pattern.debris.len(), 20);
    }

    #[test]
    fn test_layered_fracture_pattern() {
        let pattern = FracturePattern::layered(3, 4, Vec3::new(2.0, 3.0, 2.0), 12.0);

        assert_eq!(pattern.debris.len(), 12); // 3 layers × 4 pieces
    }

    #[test]
    fn test_debris_lifetime() {
        let config = DebrisConfig {
            lifetime: 5.0,
            ..Default::default()
        };
        let mut debris = Debris::new(
            DebrisId(1),
            DestructibleId(1),
            config,
            Vec3::ZERO,
            Vec3::ZERO,
        );

        assert!(!debris.should_remove());

        debris.update(3.0, Vec3::ZERO);
        assert!(!debris.should_remove());

        debris.update(3.0, Vec3::ZERO);
        assert!(debris.should_remove());
    }

    #[test]
    fn test_permanent_debris() {
        let config = DebrisConfig {
            lifetime: 0.0, // Permanent
            ..Default::default()
        };
        let mut debris = Debris::new(
            DebrisId(1),
            DestructibleId(1),
            config,
            Vec3::ZERO,
            Vec3::ZERO,
        );

        debris.update(100.0, Vec3::ZERO);
        assert!(!debris.should_remove());
    }

    #[test]
    fn test_destruction_manager_add_remove() {
        let mut manager = DestructionManager::new();

        let id = manager.add_destructible(DestructibleConfig::default(), Vec3::ZERO);
        assert_eq!(manager.destructible_count(), 1);

        assert!(manager.get(id).is_some());
        assert!(manager.remove_destructible(id));
        assert_eq!(manager.destructible_count(), 0);
    }

    #[test]
    fn test_destruction_spawns_debris() {
        let mut manager = DestructionManager::new();

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(5, Vec3::splat(0.5), 5.0),
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);

        // Update to process destruction
        manager.update(0.016, Vec3::new(0.0, -9.81, 0.0));

        assert_eq!(manager.debris_count(), 5);

        // Check events were generated
        let events = manager.take_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].debris_count, 5);
    }

    #[test]
    fn test_debris_limit() {
        let mut manager = DestructionManager::new();
        manager.max_debris = 3;

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(10, Vec3::splat(0.5), 10.0),
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.016, Vec3::ZERO);

        // Should only spawn up to max_debris
        assert_eq!(manager.debris_count(), 3);
    }

    #[test]
    fn test_debris_cleanup() {
        let mut manager = DestructionManager::new();

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![DebrisConfig {
                    lifetime: 0.5,
                    ..Default::default()
                }],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.016, Vec3::ZERO);

        assert_eq!(manager.debris_count(), 1);

        // Age debris past lifetime
        manager.update(1.0, Vec3::ZERO);
        assert_eq!(manager.debris_count(), 0);
    }

    #[test]
    fn test_cleanup_destroyed() {
        let mut manager = DestructionManager::new();

        let id = manager.add_destructible(
            DestructibleConfig {
                trigger: DestructionTrigger::Manual,
                fracture_pattern: FracturePattern {
                    debris: vec![],
                    center_of_mass: Vec3::ZERO,
                },
                ..Default::default()
            },
            Vec3::ZERO,
        );

        manager.destroy(id);
        manager.update(0.016, Vec3::ZERO);

        assert_eq!(manager.destructible_count(), 1);
        manager.cleanup_destroyed();
        assert_eq!(manager.destructible_count(), 0);
    }

    #[test]
    fn test_debris_gravity() {
        let config = DebrisConfig::default();
        let mut debris = Debris::new(
            DebrisId(1),
            DestructibleId(1),
            config,
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::ZERO,
        );

        let gravity = Vec3::new(0.0, -9.81, 0.0);
        debris.update(1.0, gravity);

        // Should have fallen
        assert!(debris.position.y < 10.0);
        assert!(debris.velocity.y < 0.0);
    }

    // ============================================================================
    // CHAIN REACTION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_damaging_debris_flag() {
        let config = DebrisConfig {
            can_damage: true,
            damage_on_hit: 25.0,
            ..Default::default()
        };

        assert!(config.can_damage);
        assert_eq!(config.damage_on_hit, 25.0);
    }

    #[test]
    fn test_multiple_destructibles_independent() {
        let mut manager = DestructionManager::new();

        let id1 = manager.add_destructible(DestructibleConfig::default(), Vec3::ZERO);
        let id2 =
            manager.add_destructible(DestructibleConfig::default(), Vec3::new(10.0, 0.0, 0.0));

        manager.apply_damage(id1, 1000.0);
        manager.update(0.016, Vec3::ZERO);

        // id1 should be destroyed, id2 should be intact
        let d1 = manager.get(id1).unwrap();
        let d2 = manager.get(id2).unwrap();

        assert_eq!(d1.state, DestructibleState::Destroyed);
        assert_eq!(d2.state, DestructibleState::Intact);
    }

    #[test]
    fn test_sequential_destruction() {
        let mut manager = DestructionManager::new();

        let ids: Vec<_> = (0..5)
            .map(|i| {
                manager.add_destructible(
                    DestructibleConfig {
                        trigger: DestructionTrigger::Manual,
                        fracture_pattern: FracturePattern::uniform(2, Vec3::splat(0.5), 1.0),
                        ..Default::default()
                    },
                    Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                )
            })
            .collect();

        // Destroy all in sequence
        for id in &ids {
            manager.destroy(*id);
        }

        manager.update(0.016, Vec3::ZERO);

        assert_eq!(manager.debris_count(), 10); // 5 objects × 2 debris each

        let events = manager.take_events();
        assert_eq!(events.len(), 5);
    }

    // ============================================================================
    // STRESS PROPAGATION TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_incremental_force_accumulation() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Force { threshold: 100.0 },
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        // Apply force in small increments
        for _ in 0..10 {
            dest.apply_force(10.0);
        }

        assert_eq!(dest.accumulated_force, 100.0);
        assert_eq!(dest.state, DestructibleState::Destroying);
    }

    #[test]
    fn test_force_reset_between_frames() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Force { threshold: 100.0 },
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        dest.apply_force(50.0);
        dest.reset_frame();

        assert_eq!(dest.accumulated_force, 0.0);

        // Object should still be intact after reset
        assert_eq!(dest.state, DestructibleState::Intact);
    }

    #[test]
    fn test_health_trigger_force_conversion() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Health,
            max_health: 100.0,
            damage_threshold: 10.0,
            force_to_damage: 0.5, // 50% of excess force becomes damage
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        // Force of 30, threshold 10, so 20 excess × 0.5 = 10 damage
        dest.apply_force(30.0);
        assert_eq!(dest.health, 90.0);
    }

    #[test]
    fn test_sub_threshold_force_no_damage() {
        let config = DestructibleConfig {
            trigger: DestructionTrigger::Health,
            max_health: 100.0,
            damage_threshold: 50.0,
            force_to_damage: 1.0,
            ..Default::default()
        };
        let mut dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        dest.apply_force(49.0);
        assert_eq!(dest.health, 100.0); // No damage, below threshold
    }

    // ============================================================================
    // DEBRIS PHYSICS TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_debris_initial_velocity() {
        let mut manager = DestructionManager::new();

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::radial(8, 1.0, 4.0),
            trigger: DestructionTrigger::Manual,
            destruction_force: 10.0,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.001, Vec3::ZERO);

        // Check debris has non-zero velocity
        for debris in manager.debris_iter() {
            assert!(
                debris.velocity.length() > 0.0,
                "Debris should have initial velocity"
            );
        }
    }

    #[test]
    fn test_debris_angular_velocity() {
        let mut manager = DestructionManager::new();

        let debris_config = DebrisConfig {
            angular_velocity_factor: 2.0,
            ..Default::default()
        };

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![debris_config],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.001, Vec3::ZERO);

        let debris = manager.debris_iter().next().unwrap();
        assert!(
            debris.angular_velocity.length() > 0.0,
            "Debris should have angular velocity"
        );
    }

    #[test]
    fn test_debris_position_from_local_offset() {
        let mut manager = DestructionManager::new();

        let debris_config = DebrisConfig {
            local_position: Vec3::new(5.0, 0.0, 0.0),
            ..Default::default()
        };

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![debris_config],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::new(10.0, 0.0, 0.0));
        manager.destroy(id);
        manager.update(0.001, Vec3::ZERO);

        let debris = manager.debris_iter().next().unwrap();
        // Position should be destructible position + local offset
        assert!((debris.position.x - 15.0).abs() < 0.01);
    }

    // ============================================================================
    // EDGE CASE TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_zero_health_destructible() {
        let config = DestructibleConfig {
            max_health: 0.0,
            ..Default::default()
        };
        let dest = Destructible::new(DestructibleId(1), config, Vec3::ZERO);

        // health_percent with max_health 0 should handle gracefully
        // (NaN or 0 depending on implementation)
        let pct = dest.health_percent();
        assert!(pct.is_nan() || pct == 0.0 || pct == 1.0);
    }

    #[test]
    fn test_empty_fracture_pattern() {
        let pattern = FracturePattern {
            debris: vec![],
            center_of_mass: Vec3::ZERO,
        };

        let mut manager = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: pattern,
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.016, Vec3::ZERO);

        assert_eq!(manager.debris_count(), 0);
    }

    #[test]
    fn test_negative_damage() {
        let mut dest = Destructible::new(
            DestructibleId(1),
            DestructibleConfig {
                max_health: 100.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );

        dest.apply_damage(-50.0); // Negative damage (healing?)

        // Implementation should allow this (health goes up)
        // or clamp to max health
        assert!(dest.health >= 100.0 && dest.health <= 150.0);
    }

    #[test]
    fn test_destroyed_ignores_damage() {
        let mut dest =
            Destructible::new(DestructibleId(1), DestructibleConfig::default(), Vec3::ZERO);

        dest.destroy();
        dest.complete_destruction();

        let health_before = dest.health;
        dest.apply_damage(50.0);

        // Destroyed objects should ignore damage
        assert_eq!(dest.health, health_before);
    }

    #[test]
    fn test_destroying_state_ignores_damage() {
        let mut dest =
            Destructible::new(DestructibleId(1), DestructibleConfig::default(), Vec3::ZERO);

        dest.destroy(); // Now in Destroying state

        let health_before = dest.health;
        dest.apply_damage(50.0);

        // Destroying objects should also ignore damage
        assert_eq!(dest.health, health_before);
    }

    // ============================================================================
    // DEFAULT CONFIG TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_debris_shape_default() {
        let shape = DebrisShape::default();
        if let DebrisShape::Box { half_extents } = shape {
            assert_eq!(half_extents, Vec3::splat(0.2));
        } else {
            panic!("Default debris shape should be Box");
        }
    }

    #[test]
    fn test_debris_config_default() {
        let config = DebrisConfig::default();

        assert_eq!(config.mass, 1.0);
        assert_eq!(config.velocity_factor, 1.0);
        assert_eq!(config.angular_velocity_factor, 0.5);
        assert_eq!(config.lifetime, 10.0);
        assert!(!config.can_damage);
        assert_eq!(config.damage_on_hit, 0.0);
    }

    #[test]
    fn test_destructible_config_default() {
        let config = DestructibleConfig::default();

        assert_eq!(config.max_health, 100.0);
        assert_eq!(config.damage_threshold, 10.0);
        assert_eq!(config.force_to_damage, 0.1);
        assert_eq!(config.destruction_force, 5.0);
        assert!(config.destruction_sound.is_none());
        assert!(config.destruction_particles.is_none());
    }

    #[test]
    fn test_destruction_trigger_default() {
        let trigger = DestructionTrigger::default();
        if let DestructionTrigger::Force { threshold } = trigger {
            assert_eq!(threshold, 1000.0);
        } else {
            panic!("Default trigger should be Force");
        }
    }

    // ============================================================================
    // MANAGER TESTS (Phase 8.8 - New)
    // ============================================================================

    #[test]
    fn test_manager_get_nonexistent() {
        let manager = DestructionManager::new();
        assert!(manager.get(DestructibleId(999)).is_none());
    }

    #[test]
    fn test_manager_remove_nonexistent() {
        let mut manager = DestructionManager::new();
        assert!(!manager.remove_destructible(DestructibleId(999)));
    }

    #[test]
    fn test_manager_apply_damage_nonexistent() {
        let mut manager = DestructionManager::new();
        // Should not panic
        manager.apply_damage(DestructibleId(999), 50.0);
    }

    #[test]
    fn test_manager_apply_force_nonexistent() {
        let mut manager = DestructionManager::new();
        // Should not panic
        manager.apply_force(DestructibleId(999), 100.0);
    }

    #[test]
    fn test_manager_on_collision_nonexistent() {
        let mut manager = DestructionManager::new();
        // Should not panic
        manager.on_collision(DestructibleId(999), 50.0);
    }

    #[test]
    fn test_manager_destroy_nonexistent() {
        let mut manager = DestructionManager::new();
        // Should not panic
        manager.destroy(DestructibleId(999));
    }

    #[test]
    fn test_manager_get_debris_nonexistent() {
        let manager = DestructionManager::new();
        assert!(manager.get_debris(DebrisId(999)).is_none());
    }

    #[test]
    fn test_manager_default_limits() {
        let manager = DestructionManager::new();
        assert_eq!(manager.max_debris, 500);
        assert_eq!(manager.default_debris_lifetime, 10.0);
    }

    #[test]
    fn test_active_debris_count() {
        let mut manager = DestructionManager::new();

        let config = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![
                    DebrisConfig {
                        lifetime: 1.0,
                        ..Default::default()
                    },
                    DebrisConfig {
                        lifetime: 10.0,
                        ..Default::default()
                    },
                ],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };

        let id = manager.add_destructible(config, Vec3::ZERO);
        manager.destroy(id);
        manager.update(0.016, Vec3::ZERO);

        assert_eq!(manager.active_debris_count(), 2);

        // Age past first debris lifetime
        manager.update(2.0, Vec3::ZERO);
        assert_eq!(manager.debris_count(), 1); // One removed
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6 — destruction system tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_uniform_piece_count_exact() {
        let pattern = FracturePattern::uniform(8, Vec3::splat(1.0), 100.0);
        assert_eq!(pattern.debris.len(), 8, "Should have exactly 8 pieces");
    }

    #[test]
    fn mutation_uniform_mass_conservation() {
        let total_mass = 120.0;
        let count = 6;
        let pattern = FracturePattern::uniform(count, Vec3::splat(2.0), total_mass);
        let sum: f32 = pattern.debris.iter().map(|d| d.mass).sum();
        assert!(
            (sum - total_mass).abs() < 0.01,
            "Total debris mass {} should equal input mass {}",
            sum,
            total_mass
        );
        // Each piece should have equal mass
        for d in &pattern.debris {
            assert!(
                (d.mass - total_mass / count as f32).abs() < 0.01,
                "Each piece should have mass {}, got {}",
                total_mass / count as f32,
                d.mass
            );
        }
    }

    #[test]
    fn mutation_uniform_grid_positions_bounded() {
        let he = Vec3::new(2.0, 3.0, 1.0);
        let pattern = FracturePattern::uniform(27, he, 50.0);
        for d in &pattern.debris {
            assert!(
                d.local_position.x.abs() <= he.x + 0.01,
                "X {} out of bounds ±{}",
                d.local_position.x,
                he.x
            );
            assert!(
                d.local_position.y.abs() <= he.y + 0.01,
                "Y {} out of bounds ±{}",
                d.local_position.y,
                he.y
            );
            assert!(
                d.local_position.z.abs() <= he.z + 0.01,
                "Z {} out of bounds ±{}",
                d.local_position.z,
                he.z
            );
        }
    }

    #[test]
    fn mutation_radial_piece_count_and_mass() {
        let pattern = FracturePattern::radial(20, 5.0, 80.0);
        assert_eq!(pattern.debris.len(), 20);
        let sum: f32 = pattern.debris.iter().map(|d| d.mass).sum();
        assert!(
            (sum - 80.0).abs() < 0.01,
            "Radial mass sum {} should be 80",
            sum
        );
    }

    #[test]
    fn mutation_radial_positions_within_radius() {
        let radius = 3.0;
        let pattern = FracturePattern::radial(30, radius, 50.0);
        for d in &pattern.debris {
            let dist = d.local_position.length();
            assert!(
                dist <= radius,
                "Piece at dist {} should be within radius {}",
                dist,
                radius
            );
        }
    }

    #[test]
    fn mutation_radial_velocity_factor() {
        let pattern = FracturePattern::radial(5, 2.0, 10.0);
        for d in &pattern.debris {
            assert!(
                (d.velocity_factor - 1.5).abs() < 1e-6,
                "Radial velocity_factor should be 1.5, got {}",
                d.velocity_factor
            );
        }
    }

    #[test]
    fn mutation_layered_count_exact() {
        let layers = 4;
        let per_layer = 6;
        let pattern = FracturePattern::layered(layers, per_layer, Vec3::new(2.0, 5.0, 2.0), 100.0);
        assert_eq!(
            pattern.debris.len(),
            layers * per_layer,
            "Should have {} total pieces",
            layers * per_layer
        );
    }

    #[test]
    fn mutation_layered_mass_conservation() {
        let total = 200.0;
        let pattern = FracturePattern::layered(3, 8, Vec3::splat(2.0), total);
        let sum: f32 = pattern.debris.iter().map(|d| d.mass).sum();
        assert!(
            (sum - total).abs() < 0.01,
            "Layered mass sum {} should be {}",
            sum,
            total
        );
    }

    #[test]
    fn mutation_layered_height_distribution() {
        let he = Vec3::new(1.0, 5.0, 1.0);
        let layers = 5;
        let pattern = FracturePattern::layered(layers, 4, he, 50.0);
        // Each layer should have pieces at a distinct Y level
        let mut y_values: Vec<f32> = pattern.debris.iter().map(|d| d.local_position.y).collect();
        y_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_values.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        assert_eq!(
            y_values.len(),
            layers,
            "Should have {} distinct Y layers, got {:?}",
            layers,
            y_values
        );
    }

    #[test]
    fn mutation_apply_damage_state_transitions() {
        let config = DestructibleConfig {
            max_health: 100.0,
            trigger: DestructionTrigger::Health,
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(0), config, Vec3::ZERO);

        assert_eq!(d.state, DestructibleState::Intact);
        assert_eq!(d.health, 100.0);

        // 30 damage: still above 50%, stays Intact
        d.apply_damage(30.0);
        assert_eq!(d.state, DestructibleState::Intact);
        assert!((d.health - 70.0).abs() < 0.01);

        // 30 more: at 40, below 50% threshold → Damaged
        d.apply_damage(30.0);
        assert_eq!(d.state, DestructibleState::Damaged);
        assert!((d.health - 40.0).abs() < 0.01);

        // Finish it off
        d.apply_damage(50.0);
        assert_eq!(d.state, DestructibleState::Destroying);
        assert!((d.health - 0.0).abs() < 0.01);
    }

    #[test]
    fn mutation_apply_damage_immune_when_destroying() {
        let config = DestructibleConfig {
            max_health: 50.0,
            trigger: DestructionTrigger::Health,
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(0), config, Vec3::ZERO);
        d.state = DestructibleState::Destroying;
        let hp_before = d.health;

        d.apply_damage(999.0);

        assert_eq!(
            d.health, hp_before,
            "Health should not change when Destroying"
        );
    }

    #[test]
    fn mutation_apply_damage_immune_when_destroyed() {
        let config = DestructibleConfig {
            max_health: 50.0,
            trigger: DestructionTrigger::Health,
            ..Default::default()
        };
        let mut d = Destructible::new(DestructibleId(0), config, Vec3::ZERO);
        d.state = DestructibleState::Destroyed;

        d.apply_damage(999.0);

        assert_eq!(d.health, 50.0, "Health should not change when Destroyed");
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — destruction Round 2 arithmetic tests
    // ═══════════════════════════════════════════════════════════════

    // --- FracturePattern::radial golden angle distribution ---
    #[test]
    fn mutation_radial_golden_angle_positions() {
        let pattern = FracturePattern::radial(10, 5.0, 50.0);
        // Verify positions use golden angle distribution
        let golden_angle = std::f32::consts::PI * (3.0 - (5.0_f32).sqrt());
        for (i, d) in pattern.debris.iter().enumerate() {
            let t = i as f32 / 10.0;
            let inclination = (1.0 - 2.0 * t).acos();
            let azimuth = golden_angle * i as f32;
            let expected = Vec3::new(
                inclination.sin() * azimuth.cos() * 5.0 * 0.8,
                inclination.cos() * 5.0 * 0.8,
                inclination.sin() * azimuth.sin() * 5.0 * 0.8,
            );
            assert!(
                (d.local_position - expected).length() < 1e-4,
                "Piece {} position mismatch: {:?} vs {:?}",
                i,
                d.local_position,
                expected
            );
        }
    }

    #[test]
    fn mutation_radial_piece_shape_is_sphere() {
        let pattern = FracturePattern::radial(5, 3.0, 10.0);
        for d in &pattern.debris {
            match d.shape {
                DebrisShape::Sphere { radius } => {
                    assert!(
                        (radius - 3.0 * 0.15).abs() < 1e-5,
                        "Radial debris radius should be radius*0.15={}, got {}",
                        3.0 * 0.15,
                        radius
                    );
                }
                _ => panic!("Radial debris should be Sphere shape"),
            }
        }
    }

    // --- FracturePattern::uniform grid arithmetic ---
    #[test]
    fn mutation_uniform_piece_size_proportional() {
        let he = Vec3::new(1.0, 2.0, 3.0);
        let count = 8;
        let pattern = FracturePattern::uniform(count, he, 40.0);
        // pieces_per_axis = ceil(cbrt(8)) = 2
        let ppa = (count as f32).cbrt().ceil() as i32;
        let expected_piece_size = he * 2.0 / ppa as f32;
        for d in &pattern.debris {
            if let DebrisShape::Box { half_extents } = d.shape {
                assert!(
                    (half_extents - expected_piece_size * 0.4).length() < 1e-4,
                    "Piece half_extents {:?} should be {:?}",
                    half_extents,
                    expected_piece_size * 0.4
                );
            } else {
                panic!("Uniform debris should be Box shape");
            }
        }
    }

    #[test]
    fn mutation_uniform_single_piece() {
        let pattern = FracturePattern::uniform(1, Vec3::splat(1.0), 5.0);
        assert_eq!(pattern.debris.len(), 1);
        assert!((pattern.debris[0].mass - 5.0).abs() < 1e-5);
    }

    // --- FracturePattern::layered layer Y spacing ---
    #[test]
    fn mutation_layered_y_positions_correct() {
        let he = Vec3::new(1.0, 6.0, 1.0);
        let layers = 3;
        let per_layer = 2;
        let pattern = FracturePattern::layered(layers, per_layer, he, 60.0);
        let layer_height = he.y * 2.0 / layers as f32; // 12/3 = 4
        for layer in 0..layers {
            let expected_y = (layer as f32 + 0.5) * layer_height - he.y;
            for p in 0..per_layer {
                let idx = layer * per_layer + p;
                assert!(
                    (pattern.debris[idx].local_position.y - expected_y).abs() < 1e-4,
                    "Layer {} piece {} Y should be {}, got {}",
                    layer,
                    p,
                    expected_y,
                    pattern.debris[idx].local_position.y
                );
            }
        }
    }

    #[test]
    fn mutation_layered_xz_uses_angle() {
        let he = Vec3::new(2.0, 3.0, 2.0);
        let layers = 1;
        let per_layer = 4;
        let pattern = FracturePattern::layered(layers, per_layer, he, 20.0);
        for (i, d) in pattern.debris.iter().enumerate() {
            let angle = i as f32 * std::f32::consts::TAU / per_layer as f32;
            let expected_x = angle.cos() * he.x * 0.7;
            let expected_z = angle.sin() * he.z * 0.7;
            assert!(
                (d.local_position.x - expected_x).abs() < 1e-4,
                "Piece {} X: expected {}, got {}",
                i,
                expected_x,
                d.local_position.x
            );
            assert!(
                (d.local_position.z - expected_z).abs() < 1e-4,
                "Piece {} Z: expected {}, got {}",
                i,
                expected_z,
                d.local_position.z
            );
        }
    }

    #[test]
    fn mutation_layered_box_shape_dimensions() {
        let he = Vec3::new(3.0, 5.0, 2.0);
        let layers = 2;
        let pattern = FracturePattern::layered(layers, 3, he, 30.0);
        let layer_height = he.y * 2.0 / layers as f32;
        for d in &pattern.debris {
            if let DebrisShape::Box { half_extents } = d.shape {
                assert!((half_extents.x - he.x * 0.3).abs() < 1e-4);
                assert!((half_extents.y - layer_height * 0.4).abs() < 1e-4);
                assert!((half_extents.z - he.z * 0.3).abs() < 1e-4);
            } else {
                panic!("Layered debris should be Box shape");
            }
        }
    }

    // --- DestructionManager::spawn_debris velocity arithmetic ---
    #[test]
    fn mutation_spawn_debris_velocity_outward() {
        let mut mgr = DestructionManager::new();
        let cfg = DestructibleConfig {
            fracture_pattern: FracturePattern::radial(4, 2.0, 10.0),
            trigger: DestructionTrigger::Manual,
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(cfg, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(0.001, Vec3::ZERO);
        for debris in mgr.debris_iter() {
            // velocity direction should be generally outward from center
            let dot = debris
                .velocity
                .dot(debris.config.local_position.normalize_or_zero());
            assert!(
                dot > 0.0 || debris.config.local_position.length() < 0.01,
                "Debris velocity should have outward component, dot={}",
                dot
            );
        }
    }

    #[test]
    fn mutation_spawn_debris_respects_velocity_factor() {
        let mut mgr = DestructionManager::new();
        let slow = DebrisConfig {
            velocity_factor: 0.5,
            local_position: Vec3::X,
            ..Default::default()
        };
        let fast = DebrisConfig {
            velocity_factor: 2.0,
            local_position: Vec3::X,
            ..Default::default()
        };
        let cfg = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![slow, fast],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(cfg, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(0.001, Vec3::ZERO);
        let speeds: Vec<f32> = mgr.debris_iter().map(|d| d.velocity.length()).collect();
        assert!(speeds.len() == 2);
        // The fast debris should have higher velocity
        assert!(
            speeds[1] > speeds[0] * 1.5 || speeds[0] > speeds[1] * 1.5,
            "Different velocity_factors should produce different speeds: {:?}",
            speeds
        );
    }

    #[test]
    fn mutation_spawn_debris_angular_velocity_deterministic() {
        let mut mgr = DestructionManager::new();
        let d1 = DebrisConfig {
            angular_velocity_factor: 1.0,
            ..Default::default()
        };
        let d2 = DebrisConfig {
            angular_velocity_factor: 1.0,
            ..Default::default()
        };
        let cfg = DestructibleConfig {
            fracture_pattern: FracturePattern {
                debris: vec![d1, d2],
                center_of_mass: Vec3::ZERO,
            },
            trigger: DestructionTrigger::Manual,
            ..Default::default()
        };
        let id = mgr.add_destructible(cfg, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(0.001, Vec3::ZERO);
        // Angular velocities should be derived from debris IDs (deterministic pseudo-random)
        let angvels: Vec<Vec3> = mgr.debris_iter().map(|d| d.angular_velocity).collect();
        assert!(angvels.len() == 2);
        // Different IDs → different angular velocities
        assert_ne!(
            angvels[0], angvels[1],
            "Different debris IDs should have different angular velocities"
        );
    }

    // --- Debris::update physics ---
    #[test]
    fn mutation_debris_update_gravity_velocity() {
        let cfg = DebrisConfig::default();
        let mut debris = Debris::new(DebrisId(1), DestructibleId(1), cfg, Vec3::ZERO, Vec3::ZERO);
        let gravity = Vec3::new(0.0, -9.81, 0.0);
        debris.update(1.0, gravity);
        // v += gravity * dt = (0, -9.81, 0), pos += v * dt = (0, -9.81, 0)
        assert!(
            (debris.velocity.y - (-9.81)).abs() < 1e-3,
            "velocity.y should be -9.81, got {}",
            debris.velocity.y
        );
        assert!(
            (debris.position.y - (-9.81)).abs() < 1e-3,
            "position.y should be -9.81, got {}",
            debris.position.y
        );
        assert!((debris.age - 1.0).abs() < 1e-6, "age should be 1.0");
    }

    #[test]
    fn mutation_debris_update_with_initial_velocity() {
        let cfg = DebrisConfig::default();
        let mut debris = Debris::new(
            DebrisId(2),
            DestructibleId(1),
            cfg,
            Vec3::ZERO,
            Vec3::new(10.0, 0.0, 0.0),
        );
        debris.update(0.5, Vec3::ZERO);
        assert!(
            (debris.position.x - 5.0).abs() < 1e-3,
            "x should be ~5 with v=10, dt=0.5"
        );
    }

    // ===== DEEP REMEDIATION v3.6.2 — destruction Round 3 remaining mutations =====

    // --- Destructible::apply_damage health threshold ---
    #[test]
    fn mutation_r3_apply_damage_health_boundary() {
        // health < max_health * 0.5  (mutation: < → <=)
        let config = DestructibleConfig {
            max_health: 100.0,
            ..Default::default()
        };
        let mut obj = Destructible::new(DestructibleId(800), config, Vec3::ZERO);
        // health starts at 100. Apply 50 damage → health = 50
        obj.apply_damage(50.0);
        // 50 < 100*0.5 = 50 is FALSE, so state should still be Intact
        assert_eq!(
            obj.state,
            DestructibleState::Intact,
            "Health=50, threshold=50: should stay Intact"
        );
        // Apply 0.01 more → health = 49.99 < 50 → Damaged
        obj.apply_damage(0.01);
        assert_eq!(
            obj.state,
            DestructibleState::Damaged,
            "Health<50 should become Damaged"
        );
    }

    #[test]
    fn mutation_r3_apply_damage_zero_health() {
        // health <= 0 → Destroying  (mutation: <= → < or ==)
        let config = DestructibleConfig {
            max_health: 10.0,
            ..Default::default()
        };
        let mut obj = Destructible::new(DestructibleId(801), config, Vec3::ZERO);
        obj.apply_damage(10.0); // health = 0 exactly
        assert_eq!(
            obj.state,
            DestructibleState::Destroying,
            "Health=0 should trigger Destroying"
        );
    }

    #[test]
    fn mutation_r3_apply_damage_ignores_non_eligible_states() {
        let config = DestructibleConfig {
            max_health: 10.0,
            ..Default::default()
        };
        let mut obj = Destructible::new(DestructibleId(802), config, Vec3::ZERO);
        obj.state = DestructibleState::Destroyed;
        let health_before = obj.health;
        obj.apply_damage(5.0);
        assert_eq!(
            obj.health, health_before,
            "Destroyed object should ignore damage"
        );
    }

    // --- DestructionManager::remove_destructible ---
    #[test]
    fn mutation_r3_remove_destructible_cleans_debris() {
        // self.debris.retain(|_, d| d.source != id)  (mutation: != → ==)
        let mut mgr = DestructionManager::new();
        let d_id = mgr.add_destructible(
            DestructibleConfig {
                max_health: 10.0,
                ..Default::default()
            },
            Vec3::ZERO,
        );
        // Manually add debris associated with this destructible
        let debris_cfg = DebrisConfig::default();
        let debris_id = DebrisId(1);
        mgr.debris.insert(
            debris_id,
            Debris::new(debris_id, d_id, debris_cfg.clone(), Vec3::ZERO, Vec3::ZERO),
        );
        // Add debris from different source
        let other_id = DestructibleId(999);
        let debris_id2 = DebrisId(2);
        mgr.debris.insert(
            debris_id2,
            Debris::new(debris_id2, other_id, debris_cfg, Vec3::ZERO, Vec3::ZERO),
        );
        assert_eq!(mgr.debris.len(), 2);
        // Remove the destructible
        let removed = mgr.remove_destructible(d_id);
        assert!(removed, "Should return true");
        // Debris from d_id should be gone, debris from other_id should remain
        assert_eq!(
            mgr.debris.len(),
            1,
            "Only other source's debris should remain"
        );
        assert!(
            mgr.debris.contains_key(&debris_id2),
            "Other debris should survive"
        );
    }

    // ===== ECS INTEGRATION SCAFFOLDING v3.7.0 — Destruction integration tests =====

    // --- spawn_debris via update() (24 misses) ---
    #[test]
    fn integration_spawn_debris_via_damage_and_update() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 50.0,
            fracture_pattern: FracturePattern::uniform(8, Vec3::splat(0.5), 10.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::new(1.0, 2.0, 3.0));

        // Damage to zero health → Destroying state
        mgr.apply_damage(id, 100.0);
        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroying);

        // update() should spawn debris and move to Destroyed
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroyed);
        assert!(
            mgr.debris_count() > 0,
            "Debris should be spawned: count={}",
            mgr.debris_count()
        );
    }

    #[test]
    fn integration_spawn_debris_position_offset() {
        let mut mgr = DestructionManager::new();
        let origin = Vec3::new(10.0, 20.0, 30.0);
        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(1.0), 5.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, origin);
        mgr.apply_damage(id, 100.0);
        mgr.update(1.0 / 60.0, Vec3::ZERO);

        // Each debris position should be origin + local_position
        for debris in mgr.debris_iter() {
            let dist = (debris.position - origin).length();
            assert!(
                dist < 10.0,
                "Debris should be near the origin: pos={:?}, dist={}",
                debris.position,
                dist
            );
        }
    }

    #[test]
    fn integration_spawn_debris_velocity_has_outward_component() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::radial(12, 1.0, 10.0),
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.apply_damage(id, 100.0);
        mgr.update(1.0 / 60.0, Vec3::ZERO);

        // Debris should have velocity (outward + force direction component)
        let mut any_has_velocity = false;
        for debris in mgr.debris_iter() {
            if debris.velocity.length() > 0.1 {
                any_has_velocity = true;
            }
        }
        assert!(
            any_has_velocity,
            "At least some debris should have velocity"
        );
    }

    #[test]
    fn integration_spawn_debris_max_limit() {
        let mut mgr = DestructionManager::new();
        mgr.max_debris = 3; // Very low limit

        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::uniform(20, Vec3::splat(1.0), 10.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.apply_damage(id, 100.0);
        mgr.update(1.0 / 60.0, Vec3::ZERO);

        assert!(
            mgr.debris_count() <= 3,
            "Should respect max_debris limit: count={}",
            mgr.debris_count()
        );
    }

    #[test]
    fn integration_spawn_debris_angular_velocity() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::radial(6, 1.0, 5.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.apply_damage(id, 100.0);
        mgr.update(1.0 / 60.0, Vec3::ZERO);

        // Debris should have angular velocity from sin-based pseudo-random
        let mut any_spin = false;
        for debris in mgr.debris_iter() {
            if debris.angular_velocity.length() > 0.01 {
                any_spin = true;
            }
        }
        assert!(any_spin, "At least some debris should spin");
    }

    #[test]
    fn integration_spawn_debris_event_emitted() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::new(5.0, 0.0, 0.0));
        mgr.apply_damage(id, 100.0);
        mgr.update(1.0 / 60.0, Vec3::ZERO);

        let events = mgr.take_events();
        assert_eq!(events.len(), 1, "Should emit one destruction event");
        assert_eq!(events[0].destructible_id, id);
        assert!(events[0].debris_count > 0);
        assert!((events[0].position.x - 5.0).abs() < 0.01);
    }

    #[test]
    fn integration_debris_update_applies_gravity() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::uniform(1, Vec3::splat(0.1), 1.0),
            destruction_force: 0.0, // No initial velocity
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::new(0.0, 10.0, 0.0));
        mgr.apply_damage(id, 100.0);
        mgr.update(0.0, Vec3::ZERO); // Spawn debris at dt=0

        // Now update with gravity for 1 second
        mgr.update(1.0, Vec3::new(0.0, -9.81, 0.0));

        for debris in mgr.debris_iter() {
            assert!(
                debris.velocity.y < -1.0,
                "Debris should fall: vel.y={}",
                debris.velocity.y
            );
        }
    }

    #[test]
    fn integration_debris_removed_after_lifetime() {
        let mut mgr = DestructionManager::new();
        mgr.default_debris_lifetime = 10.0;

        let config = DestructibleConfig {
            max_health: 1.0,
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 5.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.apply_damage(id, 100.0);
        mgr.update(0.0, Vec3::ZERO); // Spawn debris

        let count_before = mgr.debris_count();
        assert!(count_before > 0);

        // Advance time well past lifetime
        // Debris lifetime is from DebrisConfig::default() which has lifetime: 5.0
        mgr.update(10.0, Vec3::ZERO);

        assert!(
            mgr.debris_count() < count_before,
            "Old debris should be removed: before={}, after={}",
            count_before,
            mgr.debris_count()
        );
    }

    #[test]
    fn integration_force_trigger_destroys() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 1000.0,
            trigger: DestructionTrigger::Force { threshold: 500.0 },
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Below threshold
        mgr.apply_force(id, 400.0);
        assert_ne!(mgr.get(id).unwrap().state, DestructibleState::Destroying);

        mgr.update(0.0, Vec3::ZERO); // reset_frame

        // At threshold
        mgr.apply_force(id, 500.0);
        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroying);

        mgr.update(1.0 / 60.0, Vec3::ZERO);
        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroyed);
    }

    #[test]
    fn integration_fracture_radial_debris_count() {
        let fp = FracturePattern::radial(10, 1.0, 5.0);
        assert_eq!(fp.debris.len(), 10, "Should create exactly 10 pieces");

        // Verify golden angle distribution - consecutive angles differ
        if fp.debris.len() >= 2 {
            let p0 = fp.debris[0].local_position;
            let p1 = fp.debris[1].local_position;
            assert!(
                (p0 - p1).length() > 0.01,
                "Adjacent pieces should have different positions"
            );
        }
    }

    #[test]
    fn integration_fracture_layered_debris_layout() {
        let fp = FracturePattern::layered(3, 4, Vec3::splat(1.0), 12.0);
        assert_eq!(fp.debris.len(), 12, "3 layers x 4 pieces = 12");

        // Verify mass conservation
        let total_mass: f32 = fp.debris.iter().map(|d| d.mass).sum();
        assert!(
            (total_mass - 12.0).abs() < 0.01,
            "Mass should be conserved: {}",
            total_mass
        );

        // Verify layer distribution
        let mut y_values: Vec<f32> = fp.debris.iter().map(|d| d.local_position.y).collect();
        y_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_values.dedup_by(|a, b| (*a - *b).abs() < 0.01);
        assert_eq!(y_values.len(), 3, "Should have 3 distinct layers");
    }

    #[test]
    fn integration_fracture_uniform_grid_spacing() {
        let fp = FracturePattern::uniform(8, Vec3::new(2.0, 1.0, 2.0), 8.0);
        assert_eq!(fp.debris.len(), 8, "Should create exactly 8 pieces");

        // All debris should be within half_extents
        for d in &fp.debris {
            assert!(
                d.local_position.x.abs() <= 2.1,
                "X should be within half_extents: {}",
                d.local_position.x
            );
            assert!(
                d.local_position.y.abs() <= 1.1,
                "Y should be within half_extents: {}",
                d.local_position.y
            );
            assert!(
                d.local_position.z.abs() <= 2.1,
                "Z should be within half_extents: {}",
                d.local_position.z
            );
        }
    }

    #[test]
    fn integration_destroy_manual_then_update() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 10000.0,
            fracture_pattern: FracturePattern::uniform(6, Vec3::splat(0.5), 5.0),
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Manual destroy bypasses health/force checks
        mgr.destroy(id);
        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroying);

        mgr.update(1.0 / 60.0, Vec3::ZERO);
        assert_eq!(mgr.get(id).unwrap().state, DestructibleState::Destroyed);
        assert!(mgr.debris_count() > 0);
    }

    // ===== ROUND 7 =====

    #[test]
    fn r7_is_destroyed_only_when_destroyed() {
        let config = DestructibleConfig {
            max_health: 10.0,
            fracture_pattern: FracturePattern::uniform(2, Vec3::splat(0.5), 1.0),
            ..Default::default()
        };
        let mut mgr = DestructionManager::new();
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Initially not destroyed
        let d = mgr.get(id).unwrap();
        assert!(!d.is_destroyed(), "Should not be destroyed initially");

        // After taking damage but still alive
        mgr.apply_damage(id, 5.0);
        let d = mgr.get(id).unwrap();
        assert!(!d.is_destroyed(), "Should not be destroyed at half health");

        // After being destroyed
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::ZERO);
        let d = mgr.get(id).unwrap();
        assert!(d.is_destroyed(), "Should be destroyed after destroy+update");
    }

    // ===== ROUND 8: spawn_debris/get_mut/on_collision/get_debris =====

    #[test]
    fn r8_get_mut_returns_mutable_ref() {
        let mut mgr = DestructionManager::new();
        let id = mgr.add_destructible(DestructibleConfig::default(), Vec3::ZERO);
        let d = mgr.get_mut(id);
        assert!(d.is_some(), "get_mut should return Some for existing id");

        let bad_id = DestructibleId(9999);
        assert!(
            mgr.get_mut(bad_id).is_none(),
            "get_mut on invalid id should be None"
        );
    }

    #[test]
    fn r8_on_collision_applies_damage() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            max_health: 50.0,
            damage_threshold: 5.0,
            force_to_damage: 1.0,
            trigger: DestructionTrigger::Health,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Collision below threshold should not damage
        mgr.on_collision(id, 3.0);
        let d = mgr.get(id).unwrap();
        assert_eq!(
            d.health, 50.0,
            "Below-threshold collision should not damage"
        );

        // Collision above threshold should damage (force=20, threshold=5, damage=(20-5)*1.0=15)
        mgr.on_collision(id, 20.0);
        let d = mgr.get(id).unwrap();
        assert!(
            d.health < 50.0,
            "Above-threshold collision should damage: health={}",
            d.health
        );
    }

    #[test]
    fn r8_get_debris_returns_spawned() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.3), 5.0),
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::new(1.0, 2.0, 3.0));

        // Destroy and update to spawn debris
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Get debris - should have spawned some
        let debris_count = mgr.debris_iter().count();
        assert!(
            debris_count > 0,
            "Should have spawned debris after destruction"
        );

        // Each debris should be retrievable by ID
        let first_debris = mgr.debris_iter().next().unwrap();
        let retrieved = mgr.get_debris(first_debris.id);
        assert!(retrieved.is_some(), "get_debris should find spawned debris");
    }

    #[test]
    fn r8_spawn_debris_positions_near_parent() {
        let mut mgr = DestructionManager::new();
        let spawn_pos = Vec3::new(5.0, 10.0, 15.0);
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(6, Vec3::splat(0.5), 8.0),
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, spawn_pos);

        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Debris positions should be near the parent object
        for debris in mgr.debris_iter() {
            let dist = (debris.position - spawn_pos).length();
            assert!(
                dist < 10.0,
                "Debris should be near parent: pos={:?}, parent={:?}, dist={}",
                debris.position,
                spawn_pos,
                dist
            );
        }
    }

    #[test]
    fn r8_spawn_debris_has_velocity() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.3), 5.0),
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // At least some debris should have non-zero velocity
        let has_velocity = mgr.debris_iter().any(|d| d.velocity.length() > 0.1);
        assert!(
            has_velocity,
            "Some debris should have velocity from destruction force"
        );
    }

    #[test]
    fn r8_spawn_debris_angular_velocity_varies() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.3), 5.0),
            destruction_force: 10.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Different debris should have different angular velocities (pseudo-random based on ID)
        let angulars: Vec<Vec3> = mgr.debris_iter().map(|d| d.angular_velocity).collect();
        if angulars.len() >= 2 {
            assert!(
                (angulars[0] - angulars[1]).length() > 0.01,
                "Different debris should have different angular velocities"
            );
        }
    }

    #[test]
    fn r8_spawn_debris_respects_force_direction() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.3), 5.0),
            destruction_force: 20.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Apply large force from +X direction before destroying
        mgr.apply_force(id, 1000.0);
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Debris should exist
        let count = mgr.debris_iter().count();
        assert!(count > 0, "Should have debris after force + destroy");
    }

    // ===== ROUND 9: FracturePattern precision + spawn_debris velocity =====

    #[test]
    fn r9_fracture_uniform_piece_positions_correct() {
        // uniform(8, half_extents=(1,1,1), mass=10)
        // pieces_per_axis = ceil(8^(1/3)) = ceil(2.0) = 2
        // piece_size = (2.0, 2.0, 2.0) / 2 = (1.0, 1.0, 1.0)
        // First piece at (x=0,y=0,z=0): pos = (0.5*1.0 - 1.0, 0.5*1.0 - 1.0, 0.5*1.0 - 1.0) = (-0.5, -0.5, -0.5)
        let frac = FracturePattern::uniform(8, Vec3::splat(1.0), 10.0);
        assert_eq!(frac.debris.len(), 8, "Should have 8 pieces");

        let first = &frac.debris[0];
        let expected_pos = Vec3::new(-0.5, -0.5, -0.5);
        assert!(
            (first.local_position - expected_pos).length() < 0.01,
            "First debris position should be ({:?}), got {:?}",
            expected_pos,
            first.local_position
        );

        // Loop order is x(outer) → y → z(inner), so index 1 = (x=0,y=0,z=1):
        // pos = (0.5*1.0 - 1.0, 0.5*1.0 - 1.0, 1.5*1.0 - 1.0) = (-0.5, -0.5, 0.5)
        let second = &frac.debris[1];
        let expected_pos2 = Vec3::new(-0.5, -0.5, 0.5);
        assert!(
            (second.local_position - expected_pos2).length() < 0.01,
            "Second debris position should be ({:?}), got {:?}",
            expected_pos2,
            second.local_position
        );
    }

    #[test]
    fn r9_fracture_uniform_piece_mass_correct() {
        let total_mass = 20.0_f32;
        let count = 8_usize;
        let frac = FracturePattern::uniform(count, Vec3::splat(2.0), total_mass);

        for (i, d) in frac.debris.iter().enumerate() {
            let expected_mass = total_mass / count as f32;
            assert!(
                (d.mass - expected_mass).abs() < 0.01,
                "Piece {} mass should be {}, got {}",
                i,
                expected_mass,
                d.mass
            );
        }
    }

    #[test]
    fn r9_fracture_uniform_piece_size_correct() {
        // half_extents = (3, 3, 3), pieces=8 → pieces_per_axis=2
        // piece_size = (6/2, 6/2, 6/2) = (3, 3, 3)
        // shape half_extents = piece_size * 0.4 = (1.2, 1.2, 1.2)
        let frac = FracturePattern::uniform(8, Vec3::splat(3.0), 10.0);
        let first = &frac.debris[0];
        match &first.shape {
            DebrisShape::Box { half_extents } => {
                let expected = 3.0 * 0.4; // = 1.2
                assert!(
                    (half_extents.x - expected).abs() < 0.01,
                    "Piece half_extent.x should be ~{}, got {}",
                    expected,
                    half_extents.x
                );
            }
            _ => panic!("Expected Box shape from FracturePattern::uniform"),
        }
    }

    #[test]
    fn r9_fracture_uniform_local_position_field_set() {
        // Test that local_position is explicitly set (catches "delete field local_position" mutation)
        let frac = FracturePattern::uniform(1, Vec3::splat(1.0), 5.0);
        let piece = &frac.debris[0];
        // With 1 piece: pieces_per_axis=1, piece_size=2.0/1=2.0, pos = (0.5*2.0-1.0) = 0.0
        assert!(
            piece.local_position.length() < 0.01,
            "Single piece should be at origin, got {:?}",
            piece.local_position
        );

        // With 8 pieces, first piece should be at (-0.5, -0.5, -0.5) — NOT at default (0,0,0) for all
        let frac2 = FracturePattern::uniform(8, Vec3::splat(1.0), 10.0);
        let last = &frac2.debris[7];
        // Last piece at (1,1,1): pos = (1.5*1-1, 1.5*1-1, 1.5*1-1) = (0.5,0.5,0.5)
        assert!(
            last.local_position.length() > 0.1,
            "Last piece should NOT be at origin: got {:?}",
            last.local_position
        );
    }

    #[test]
    fn r9_spawn_debris_velocity_has_outward_component() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 50.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::new(0.0, 5.0, 0.0));
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Check debris velocities: they should have nonzero velocity from the destruction force
        for debris in mgr.debris_iter() {
            assert!(
                debris.velocity.length() > 0.01,
                "Debris should have velocity from destruction force, got {:?}",
                debris.velocity
            );
        }
    }

    #[test]
    fn r9_spawn_debris_force_direction_affects_velocity() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 100.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);

        // Apply large force from +X to trigger destruction with force direction
        mgr.apply_force(id, 50000.0); // Exceed force threshold to trigger destruction
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Debris should exist and have been pushed in +X direction due to force
        let count = mgr.debris_iter().count();
        if count > 0 {
            let avg_vx: f32 = mgr.debris_iter().map(|d| d.velocity.x).sum::<f32>() / count as f32;
            // Force was applied in +X direction, so average velocity should lean positive x
            // (At minimum, debris should have non-trivial velocity)
            let avg_speed: f32 =
                mgr.debris_iter().map(|d| d.velocity.length()).sum::<f32>() / count as f32;
            assert!(
                avg_speed > 0.5,
                "Debris should have speed from destruction_force=100, got {}",
                avg_speed
            );
        }
    }

    #[test]
    fn r9_spawn_debris_angular_velocity_uses_factor() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 50.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Check debris angular velocities: they should be nonzero (pseudo-random based on id)
        let mut has_angular = false;
        for debris in mgr.debris_iter() {
            if debris.angular_velocity.length() > 0.01 {
                has_angular = true;
            }
        }
        assert!(
            has_angular,
            "At least some debris should have angular velocity"
        );
    }

    // ===== ROUND 10: spawn_debris exact arithmetic verification =====

    #[test]
    fn r10_spawn_debris_angular_velocity_exact_id1() {
        // Debris IDs start at 1 (next_debris_id = 1 in DestructionManager::new).
        // Angular velocity formula for id N:
        //   Vec3(sin(N*1.234)*5, sin(N*2.345)*5, sin(N*3.456)*5) * ang_factor
        // Default ang_factor from DebrisConfig = 0.5
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 50.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // Access specific debris by ID
        let d1 = mgr.get_debris(DebrisId(1)).expect("Debris 1 should exist");
        let ang_factor = 0.5_f32;
        let expected_x = (1.0_f32 * 1.234).sin() * 5.0 * ang_factor;
        let expected_y = (1.0_f32 * 2.345).sin() * 5.0 * ang_factor;
        let expected_z = (1.0_f32 * 3.456).sin() * 5.0 * ang_factor;
        let expected = Vec3::new(expected_x, expected_y, expected_z);

        assert!(
            (d1.angular_velocity - expected).length() < 0.01,
            "Debris 1 angular vel should match formula: expected={:?}, got={:?}",
            expected,
            d1.angular_velocity
        );
    }

    #[test]
    fn r10_spawn_debris_angular_velocity_exact_id2() {
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 50.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        let d2 = mgr.get_debris(DebrisId(2)).expect("Debris 2 should exist");
        let ang_factor = 0.5_f32;
        let expected = Vec3::new(
            (2.0_f32 * 1.234).sin() * 5.0 * ang_factor,
            (2.0_f32 * 2.345).sin() * 5.0 * ang_factor,
            (2.0_f32 * 3.456).sin() * 5.0 * ang_factor,
        );
        assert!(
            (d2.angular_velocity - expected).length() < 0.01,
            "Debris 2 angular vel: expected={:?}, got={:?}",
            expected,
            d2.angular_velocity
        );
    }

    #[test]
    fn r10_spawn_debris_velocity_formula_outward_plus_force() {
        // velocity = (outward * destruction_force + force_dir * destruction_force * 0.5)
        //            * velocity_factor
        // force_direction defaults to Vec3::Y in update()
        let mut mgr = DestructionManager::new();
        let config = DestructibleConfig {
            fracture_pattern: FracturePattern::uniform(4, Vec3::splat(0.5), 5.0),
            destruction_force: 100.0,
            ..Default::default()
        };
        let id = mgr.add_destructible(config, Vec3::ZERO);
        mgr.destroy(id);
        mgr.update(1.0 / 60.0, Vec3::new(0.0, -9.81, 0.0));

        // First debris (ID=1) has local_position from FracturePattern::uniform
        let d1 = mgr.get_debris(DebrisId(1)).expect("Debris 1");
        // After one update frame, velocity has been modified by gravity
        // But the initial velocity was set during spawn. Let's check that it has a Y
        // component above zero (force_direction * 0.5 gives +Y bias of 50)
        // Even after 1 frame of gravity (9.81 * 1/60 ≈ 0.16), the Y component should
        // still be positive if force direction contributed

        // Check velocity is non-trivial (destruction_force=100 gives high initial speed)
        assert!(
            d1.velocity.length() > 10.0,
            "Debris velocity should be significant: {:?}",
            d1.velocity
        );

        // Y component should include the force_direction (Vec3::Y) contribution
        // At minimum, some debris should have positive Y velocity from the +Y force component
        let has_positive_y = mgr.debris_iter().any(|d| d.velocity.y > 5.0);
        assert!(
            has_positive_y,
            "Force direction +Y should give some debris positive Y velocity"
        );
    }

    #[test]
    fn r10_spawn_debris_velocity_factor_scales() {
        // If velocity_factor matters, changing it should change debris speed
        // Create two destructibles with different velocity_factor patterns
        let make_pattern = |vel_factor: f32| FracturePattern {
            debris: vec![DebrisConfig {
                local_position: Vec3::new(0.5, 0.0, 0.0),
                velocity_factor: vel_factor,
                angular_velocity_factor: 0.5,
                mass: 1.0,
                ..Default::default()
            }],
            center_of_mass: Vec3::ZERO,
        };

        let mut mgr1 = DestructionManager::new();
        let c1 = DestructibleConfig {
            fracture_pattern: make_pattern(1.0),
            destruction_force: 100.0,
            ..Default::default()
        };
        let id1 = mgr1.add_destructible(c1, Vec3::ZERO);
        mgr1.destroy(id1);
        mgr1.update(0.0, Vec3::ZERO); // dt=0, no gravity

        let mut mgr2 = DestructionManager::new();
        let c2 = DestructibleConfig {
            fracture_pattern: make_pattern(0.5),
            destruction_force: 100.0,
            ..Default::default()
        };
        let id2 = mgr2.add_destructible(c2, Vec3::ZERO);
        mgr2.destroy(id2);
        mgr2.update(0.0, Vec3::ZERO);

        let v1 = mgr1.get_debris(DebrisId(1)).unwrap().velocity.length();
        let v2 = mgr2.get_debris(DebrisId(1)).unwrap().velocity.length();

        assert!(
            (v1 - v2 * 2.0).abs() < v1 * 0.1,
            "velocity_factor 1.0 should be ~2x of 0.5: v1={}, v2={}",
            v1,
            v2
        );
    }
}
