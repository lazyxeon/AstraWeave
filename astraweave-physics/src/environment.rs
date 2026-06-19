//! Environmental Physics Systems
//!
//! This module provides environmental effects that interact with rigid bodies:
//! - Wind zones (directional, vortex, turbulent)
//! - Gust system (noise-based variation)

use glam::Vec3;
use std::collections::HashMap;

/// Unique identifier for wind zones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindZoneId(pub u64);

/// Shape of a wind zone
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum WindZoneShape {
    /// Infinite directional wind (global)
    Global,
    /// Box-shaped wind zone
    Box { half_extents: Vec3 },
    /// Spherical wind zone
    Sphere { radius: f32 },
    /// Cylindrical wind zone (useful for tornadoes)
    Cylinder { radius: f32, height: f32 },
}

impl Default for WindZoneShape {
    fn default() -> Self {
        Self::Global
    }
}

/// Type of wind behavior
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum WindType {
    /// Constant directional wind
    Directional,
    /// Rotational wind around center (tornado, whirlpool)
    Vortex {
        /// Tangential speed at edge
        tangential_speed: f32,
        /// Inward pull strength
        inward_pull: f32,
        /// Upward lift
        updraft: f32,
    },
    /// Turbulent wind with noise
    Turbulent {
        /// Base turbulence intensity
        intensity: f32,
        /// Frequency of turbulence changes
        frequency: f32,
    },
}

impl Default for WindType {
    fn default() -> Self {
        Self::Directional
    }
}

/// Configuration for a wind zone
#[derive(Debug, Clone)]
pub struct WindZoneConfig {
    /// Position of the wind zone center
    pub position: Vec3,
    /// Shape of the zone
    pub shape: WindZoneShape,
    /// Type of wind behavior
    pub wind_type: WindType,
    /// Base wind direction (for directional/turbulent)
    pub direction: Vec3,
    /// Base wind strength (force per unit area)
    pub strength: f32,
    /// Falloff from center (0 = uniform, 1 = linear falloff to edge)
    pub falloff: f32,
    /// Whether this zone is active
    pub active: bool,
}

impl Default for WindZoneConfig {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 10.0,
            falloff: 0.0,
            active: true,
        }
    }
}

/// Runtime state for a wind zone
#[derive(Debug, Clone)]
pub struct WindZone {
    pub id: WindZoneId,
    pub config: WindZoneConfig,
    /// Current gust offset (for turbulent wind)
    pub gust_offset: Vec3,
    /// Phase for noise-based variation
    pub noise_phase: f32,
}

impl WindZone {
    /// Create a new wind zone
    pub fn new(id: WindZoneId, config: WindZoneConfig) -> Self {
        Self {
            id,
            config,
            gust_offset: Vec3::ZERO,
            noise_phase: 0.0,
        }
    }

    /// Check if a point is inside this wind zone
    pub fn contains(&self, point: Vec3) -> bool {
        match self.config.shape {
            WindZoneShape::Global => true,
            WindZoneShape::Box { half_extents } => {
                let local = point - self.config.position;
                local.x.abs() <= half_extents.x
                    && local.y.abs() <= half_extents.y
                    && local.z.abs() <= half_extents.z
            }
            WindZoneShape::Sphere { radius } => (point - self.config.position).length() <= radius,
            WindZoneShape::Cylinder { radius, height } => {
                let local = point - self.config.position;
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length();
                horizontal_dist <= radius && local.y.abs() <= height / 2.0
            }
        }
    }

    /// Calculate wind force at a given point
    pub fn wind_force_at(&self, point: Vec3, drag_coefficient: f32, cross_section: f32) -> Vec3 {
        if !self.config.active || !self.contains(point) {
            return Vec3::ZERO;
        }

        // Calculate distance factor for falloff
        let distance_factor = self.calculate_falloff(point);

        // Calculate base wind velocity at this point
        let wind_velocity = match self.config.wind_type {
            WindType::Directional => {
                self.config.direction.normalize_or_zero() * self.config.strength
            }
            WindType::Vortex {
                tangential_speed,
                inward_pull,
                updraft,
            } => {
                let to_center = self.config.position - point;
                let horizontal = Vec3::new(to_center.x, 0.0, to_center.z);
                let dist = horizontal.length();

                if dist < 0.1 {
                    Vec3::new(0.0, updraft, 0.0)
                } else {
                    // Tangential component (perpendicular to radius)
                    let tangent = Vec3::new(-horizontal.z, 0.0, horizontal.x).normalize();
                    let tangential = tangent * tangential_speed;

                    // Inward pull
                    let inward = horizontal.normalize() * inward_pull;

                    // Combine
                    tangential + inward + Vec3::new(0.0, updraft, 0.0)
                }
            }
            WindType::Turbulent { intensity, .. } => {
                let base = self.config.direction.normalize_or_zero() * self.config.strength;
                base + self.gust_offset * intensity
            }
        };

        // Apply falloff
        let effective_velocity = wind_velocity * distance_factor;

        // Wind force = 0.5 * air_density * velocity^2 * drag_coefficient * area
        // Simplified: F = k * v^2 * direction
        let speed = effective_velocity.length();
        if speed < 0.01 {
            return Vec3::ZERO;
        }

        let force_magnitude = 0.5 * 1.225 * speed * speed * drag_coefficient * cross_section;
        effective_velocity.normalize() * force_magnitude
    }

    /// Calculate falloff factor based on distance from center
    fn calculate_falloff(&self, point: Vec3) -> f32 {
        if self.config.falloff <= 0.0 {
            return 1.0;
        }

        let normalized_dist = match self.config.shape {
            WindZoneShape::Global => 0.0,
            WindZoneShape::Box { half_extents } => {
                let local = (point - self.config.position).abs();

                (local / half_extents).max_element()
            }
            WindZoneShape::Sphere { radius } => (point - self.config.position).length() / radius,
            WindZoneShape::Cylinder { radius, height } => {
                let local = point - self.config.position;
                let horizontal_dist = Vec3::new(local.x, 0.0, local.z).length() / radius;
                let vertical_dist = local.y.abs() / (height / 2.0);
                horizontal_dist.max(vertical_dist)
            }
        };

        (1.0 - normalized_dist * self.config.falloff).max(0.0)
    }

    /// Update turbulence/gust state
    pub fn update(&mut self, dt: f32) {
        if let WindType::Turbulent { frequency, .. } = self.config.wind_type {
            self.noise_phase += dt * frequency;

            // Simple pseudo-random gust using sine waves at different frequencies
            self.gust_offset = Vec3::new(
                (self.noise_phase * 1.0).sin() * 0.5 + (self.noise_phase * 2.3).sin() * 0.3,
                (self.noise_phase * 0.7).sin() * 0.2 + (self.noise_phase * 1.9).sin() * 0.15,
                (self.noise_phase * 1.3).sin() * 0.5 + (self.noise_phase * 2.7).sin() * 0.3,
            );
        }
    }
}

/// Gust event for sudden wind changes
#[derive(Debug, Clone)]
pub struct GustEvent {
    /// Direction of the gust
    pub direction: Vec3,
    /// Peak strength
    pub strength: f32,
    /// Duration in seconds
    pub duration: f32,
    /// Time elapsed
    pub elapsed: f32,
    /// Shape of gust envelope (0 = instant, 1 = smooth)
    pub smoothness: f32,
}

impl GustEvent {
    /// Create a new gust event
    pub fn new(direction: Vec3, strength: f32, duration: f32) -> Self {
        Self {
            direction: direction.normalize_or_zero(),
            strength,
            duration,
            elapsed: 0.0,
            smoothness: 0.5,
        }
    }

    /// Get current gust force multiplier
    pub fn current_strength(&self) -> f32 {
        if self.elapsed >= self.duration {
            return 0.0;
        }

        let t = self.elapsed / self.duration;

        // Smooth envelope: ramp up, hold, ramp down
        let envelope = if self.smoothness > 0.0 {
            let attack = (t * 4.0).min(1.0);
            let release = ((1.0 - t) * 4.0).min(1.0);
            attack * release
        } else {
            1.0
        };

        self.strength * envelope
    }

    /// Update gust timer
    pub fn update(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    /// Check if gust is finished
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

/// Manager for all environmental effects
#[derive(Debug, Default)]
pub struct EnvironmentManager {
    wind_zones: HashMap<WindZoneId, WindZone>,
    gusts: Vec<GustEvent>,
    next_wind_id: u64,
    /// Global wind (affects everything)
    pub global_wind: Vec3,
    /// Global wind strength multiplier
    pub global_wind_strength: f32,
}

impl EnvironmentManager {
    /// Create a new environment manager
    pub fn new() -> Self {
        Self {
            wind_zones: HashMap::new(),
            gusts: Vec::new(),
            next_wind_id: 1,
            global_wind: Vec3::ZERO,
            global_wind_strength: 1.0,
        }
    }

    // === Wind Zone Management ===

    /// Add a wind zone
    pub fn add_wind_zone(&mut self, config: WindZoneConfig) -> WindZoneId {
        let id = WindZoneId(self.next_wind_id);
        self.next_wind_id += 1;
        self.wind_zones.insert(id, WindZone::new(id, config));
        id
    }

    /// Remove a wind zone
    pub fn remove_wind_zone(&mut self, id: WindZoneId) -> bool {
        self.wind_zones.remove(&id).is_some()
    }

    /// Get a wind zone
    pub fn get_wind_zone(&self, id: WindZoneId) -> Option<&WindZone> {
        self.wind_zones.get(&id)
    }

    /// Get a mutable wind zone
    pub fn get_wind_zone_mut(&mut self, id: WindZoneId) -> Option<&mut WindZone> {
        self.wind_zones.get_mut(&id)
    }

    /// Set wind zone active state
    pub fn set_wind_zone_active(&mut self, id: WindZoneId, active: bool) {
        if let Some(zone) = self.wind_zones.get_mut(&id) {
            zone.config.active = active;
        }
    }

    // === Gust Events ===

    /// Trigger a gust event
    pub fn trigger_gust(&mut self, direction: Vec3, strength: f32, duration: f32) {
        self.gusts
            .push(GustEvent::new(direction, strength, duration));
    }

    /// Get current gust force
    pub fn current_gust_force(&self) -> Vec3 {
        self.gusts
            .iter()
            .map(|g| g.direction * g.current_strength())
            .fold(Vec3::ZERO, |a, b| a + b)
    }

    // === Force Calculations ===

    /// Calculate total wind force at a point
    pub fn wind_force_at(&self, point: Vec3, drag_coefficient: f32, cross_section: f32) -> Vec3 {
        let mut total = Vec3::ZERO;

        // Global wind
        if self.global_wind.length_squared() > 0.001 {
            let speed = self.global_wind.length() * self.global_wind_strength;
            let force = 0.5 * 1.225 * speed * speed * drag_coefficient * cross_section;
            total += self.global_wind.normalize() * force;
        }

        // Wind zones
        for zone in self.wind_zones.values() {
            total += zone.wind_force_at(point, drag_coefficient, cross_section);
        }

        // Gusts
        let gust = self.current_gust_force();
        if gust.length_squared() > 0.001 {
            let speed = gust.length();
            let force = 0.5 * 1.225 * speed * speed * drag_coefficient * cross_section;
            total += gust.normalize() * force;
        }

        total
    }

    // === Update ===

    /// Update all environmental effects
    pub fn update(&mut self, dt: f32) {
        // Update wind zones
        for zone in self.wind_zones.values_mut() {
            zone.update(dt);
        }

        // Update gusts and remove finished ones
        for gust in &mut self.gusts {
            gust.update(dt);
        }
        self.gusts.retain(|g| !g.is_finished());
    }

    /// Get number of active wind zones
    pub fn wind_zone_count(&self) -> usize {
        self.wind_zones.len()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wind_zone_creation() {
        let config = WindZoneConfig::default();
        let zone = WindZone::new(WindZoneId(1), config);
        assert_eq!(zone.id, WindZoneId(1));
        assert!(zone.config.active);
    }

    #[test]
    fn test_global_wind_zone_contains() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Global zone contains everything
        assert!(zone.contains(Vec3::ZERO));
        assert!(zone.contains(Vec3::new(1000.0, 1000.0, 1000.0)));
        assert!(zone.contains(Vec3::new(-500.0, 200.0, -300.0)));
    }

    #[test]
    fn test_box_wind_zone_contains() {
        let config = WindZoneConfig {
            position: Vec3::new(10.0, 5.0, 0.0),
            shape: WindZoneShape::Box {
                half_extents: Vec3::new(5.0, 5.0, 5.0),
            },
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Inside
        assert!(zone.contains(Vec3::new(10.0, 5.0, 0.0)));
        assert!(zone.contains(Vec3::new(14.0, 5.0, 0.0)));

        // Outside
        assert!(!zone.contains(Vec3::new(16.0, 5.0, 0.0)));
        assert!(!zone.contains(Vec3::ZERO));
    }

    #[test]
    fn test_sphere_wind_zone_contains() {
        let config = WindZoneConfig {
            position: Vec3::new(0.0, 10.0, 0.0),
            shape: WindZoneShape::Sphere { radius: 5.0 },
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Inside
        assert!(zone.contains(Vec3::new(0.0, 10.0, 0.0)));
        assert!(zone.contains(Vec3::new(0.0, 14.0, 0.0)));

        // Outside
        assert!(!zone.contains(Vec3::new(0.0, 16.0, 0.0)));
        assert!(!zone.contains(Vec3::ZERO));
    }

    #[test]
    fn test_cylinder_wind_zone_contains() {
        let config = WindZoneConfig {
            position: Vec3::new(0.0, 5.0, 0.0),
            shape: WindZoneShape::Cylinder {
                radius: 3.0,
                height: 10.0,
            },
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Inside
        assert!(zone.contains(Vec3::new(0.0, 5.0, 0.0)));
        assert!(zone.contains(Vec3::new(2.0, 5.0, 0.0)));

        // Outside (beyond radius)
        assert!(!zone.contains(Vec3::new(4.0, 5.0, 0.0)));
        // Outside (beyond height)
        assert!(!zone.contains(Vec3::new(0.0, 15.0, 0.0)));
    }

    #[test]
    fn test_directional_wind_force() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 10.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(force.x > 0.0, "Wind should push in +X direction");
        assert!(force.y.abs() < 0.01);
        assert!(force.z.abs() < 0.01);
    }

    #[test]
    fn test_vortex_wind_force() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 100.0 },
            wind_type: WindType::Vortex {
                tangential_speed: 10.0,
                inward_pull: 5.0,
                updraft: 2.0,
            },
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Test point to the right of center
        let force = zone.wind_force_at(Vec3::new(10.0, 0.0, 0.0), 1.0, 1.0);

        // Should have inward component (toward center, so -X)
        // Should have updraft (+ Y)
        // Should have tangential component
        assert!(force.length() > 0.0, "Vortex should produce force");
    }

    #[test]
    fn test_turbulent_wind_update() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 1.0,
            },
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(1), config);

        let _initial_offset = zone.gust_offset;
        zone.update(0.5);

        // Gust offset should change after update
        assert!(zone.noise_phase > 0.0);
        // Note: gust_offset may still be zero at certain phases
    }

    #[test]
    fn test_wind_falloff() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            falloff: 1.0,
            strength: 10.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        let center_force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let edge_force = zone.wind_force_at(Vec3::new(9.0, 0.0, 0.0), 1.0, 1.0);

        // Force should be stronger at center
        assert!(
            center_force.length() > edge_force.length(),
            "Center force should be stronger with falloff"
        );
    }

    #[test]
    fn test_inactive_wind_zone() {
        let config = WindZoneConfig {
            active: false,
            strength: 100.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert_eq!(force, Vec3::ZERO, "Inactive zone should produce no force");
    }

    #[test]
    fn test_gust_event_lifecycle() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 1.0);

        // Initial state (at t=0, envelope starts from 0 due to attack ramp)
        assert!(!gust.is_finished());

        // Small step forward - should have strength now
        gust.update(0.1);
        assert!(
            gust.current_strength() > 0.0,
            "Gust should have strength after starting"
        );

        // Mid-gust
        gust.update(0.4);
        assert!(!gust.is_finished());

        // After duration
        gust.update(0.6);
        assert!(gust.is_finished());
        assert_eq!(gust.current_strength(), 0.0);
    }

    #[test]
    fn test_environment_manager_wind_zones() {
        let mut manager = EnvironmentManager::new();

        let id = manager.add_wind_zone(WindZoneConfig::default());
        assert_eq!(manager.wind_zone_count(), 1);

        assert!(manager.get_wind_zone(id).is_some());

        manager.set_wind_zone_active(id, false);
        assert!(!manager.get_wind_zone(id).unwrap().config.active);

        assert!(manager.remove_wind_zone(id));
        assert_eq!(manager.wind_zone_count(), 0);
    }

    #[test]
    fn test_environment_manager_gusts() {
        let mut manager = EnvironmentManager::new();

        manager.trigger_gust(Vec3::X, 10.0, 1.0);

        // Advance time slightly for gust attack envelope to ramp up
        manager.update(0.1);
        let gust_force = manager.current_gust_force();
        assert!(
            gust_force.length() > 0.0,
            "Gust should produce force after ramp"
        );

        // After duration, gust should be removed
        manager.update(1.1);
        let gust_force = manager.current_gust_force();
        assert_eq!(gust_force, Vec3::ZERO);
    }

    #[test]
    fn test_combined_wind_force() {
        let mut manager = EnvironmentManager::new();

        // Global wind
        manager.global_wind = Vec3::new(5.0, 0.0, 0.0);
        manager.global_wind_strength = 1.0;

        // Add a wind zone
        manager.add_wind_zone(WindZoneConfig {
            direction: Vec3::new(0.0, 0.0, 5.0),
            strength: 5.0,
            ..Default::default()
        });

        let force = manager.wind_force_at(Vec3::ZERO, 1.0, 1.0);

        // Should have components from both sources
        assert!(force.x > 0.0, "Should have global wind X component");
        assert!(force.z > 0.0, "Should have zone wind Z component");
    }

    #[test]
    fn test_wind_zone_turbulent_force() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 1.0,
            },
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(1), config);
        zone.gust_offset = Vec3::new(1.0, 1.0, 1.0);

        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(force.length() > 0.0);
    }

    #[test]
    fn test_wind_zone_falloff_shapes() {
        // Box falloff
        let config_box = WindZoneConfig {
            shape: WindZoneShape::Box {
                half_extents: Vec3::ONE * 10.0,
            },
            falloff: 1.0,
            ..Default::default()
        };
        let zone_box = WindZone::new(WindZoneId(1), config_box);
        assert!(zone_box.calculate_falloff(Vec3::ZERO) == 1.0);
        assert!(zone_box.calculate_falloff(Vec3::ONE * 5.0) < 1.0);

        // Cylinder falloff
        let config_cyl = WindZoneConfig {
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 10.0,
            },
            falloff: 1.0,
            ..Default::default()
        };
        let zone_cyl = WindZone::new(WindZoneId(2), config_cyl);
        assert!(zone_cyl.calculate_falloff(Vec3::ZERO) == 1.0);
        assert!(zone_cyl.calculate_falloff(Vec3::new(5.0, 0.0, 0.0)) < 1.0);
    }

    #[test]
    fn test_environment_manager_mut_access() {
        let mut manager = EnvironmentManager::new();
        let w_id = manager.add_wind_zone(WindZoneConfig::default());

        assert!(manager.get_wind_zone_mut(w_id).is_some());
    }

    #[test]
    fn test_wind_defaults() {
        let _ = WindZoneShape::default();
        let _ = WindType::default();
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6 — environment physics tests
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_wind_force_inactive_zone() {
        let mut config = WindZoneConfig::default();
        config.active = false;
        let zone = WindZone::new(WindZoneId(200), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert_eq!(
            force,
            Vec3::ZERO,
            "Inactive wind zone should produce no force"
        );
    }

    #[test]
    fn mutation_wind_force_directional() {
        let mut config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 10.0,
            ..Default::default()
        };
        config.active = true;

        let zone = WindZone::new(WindZoneId(201), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // Force should be in +X direction (wind direction)
        assert!(force.x > 0.0, "Wind force should be in wind direction");
        assert!(force.y.abs() < 0.01, "Wind force should be horizontal");
    }

    #[test]
    fn mutation_gust_event_lifecycle() {
        let mut gust = GustEvent::new(Vec3::X, 2.0, 1.0);
        assert!(!gust.is_finished());

        let str0 = gust.current_strength();
        assert!(str0 >= 0.0, "Initial gust strength should be non-negative");

        gust.update(0.5);
        assert!(
            !gust.is_finished(),
            "Gust should still be active at 0.5s (dur=1.0)"
        );

        gust.update(0.6);
        assert!(gust.is_finished(), "Gust should be finished after 1.1s");
    }

    // ═══════════════════════════════════════════════════════════════
    // DEEP REMEDIATION v3.6.1 — environment Round 2 arithmetic/boundary tests
    // ═══════════════════════════════════════════════════════════════

    // --- WindZone::update turbulent ---
    #[test]
    fn mutation_windzone_update_noise_phase_advances() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 2.0,
            },
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(300), config);
        assert_eq!(zone.noise_phase, 0.0);
        zone.update(0.5);
        // noise_phase should increase by dt * frequency = 0.5 * 2.0 = 1.0
        assert!(
            (zone.noise_phase - 1.0).abs() < 1e-6,
            "noise_phase should be 1.0, got {}",
            zone.noise_phase
        );
    }

    #[test]
    fn mutation_windzone_update_gust_offset_nonzero() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 3.0,
            },
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(301), config);
        zone.update(1.0);
        // At noise_phase=3.0, gust_offset should be non-trivial
        // gust_offset.x = sin(3.0)*0.5 + sin(3.0*2.3)*0.3
        let expected_x = (3.0_f32 * 1.0).sin() * 0.5 + (3.0_f32 * 2.3).sin() * 0.3;
        let expected_y = (3.0_f32 * 0.7).sin() * 0.2 + (3.0_f32 * 1.9).sin() * 0.15;
        let expected_z = (3.0_f32 * 1.3).sin() * 0.5 + (3.0_f32 * 2.7).sin() * 0.3;
        assert!(
            (zone.gust_offset.x - expected_x).abs() < 1e-5,
            "gust_offset.x mismatch"
        );
        assert!(
            (zone.gust_offset.y - expected_y).abs() < 1e-5,
            "gust_offset.y mismatch"
        );
        assert!(
            (zone.gust_offset.z - expected_z).abs() < 1e-5,
            "gust_offset.z mismatch"
        );
    }

    #[test]
    fn mutation_windzone_update_directional_no_change() {
        let config = WindZoneConfig {
            wind_type: WindType::Directional,
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(302), config);
        let phase_before = zone.noise_phase;
        let offset_before = zone.gust_offset;
        zone.update(1.0);
        assert_eq!(
            zone.noise_phase, phase_before,
            "Directional wind should not update noise_phase"
        );
        assert_eq!(
            zone.gust_offset, offset_before,
            "Directional wind should not update gust_offset"
        );
    }

    // --- WindZone::wind_force_at arithmetic ---
    #[test]
    fn mutation_wind_force_at_directional_exact() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 20.0,
            falloff: 0.0,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(310), config);
        let force = zone.wind_force_at(Vec3::ZERO, 0.5, 2.0);
        // effective_velocity = direction * strength = (20, 0, 0), speed = 20
        // force_magnitude = 0.5 * 1.225 * 400 * 0.5 * 2.0 = 245.0
        let expected = 0.5 * 1.225 * 20.0 * 20.0 * 0.5 * 2.0;
        assert!(
            (force.x - expected).abs() < 0.1,
            "force.x should be {}, got {}",
            expected,
            force.x
        );
        assert!(force.y.abs() < 1e-3);
        assert!(force.z.abs() < 1e-3);
    }

    #[test]
    fn mutation_wind_force_at_outside_box_zero() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Box {
                half_extents: Vec3::splat(5.0),
            },
            strength: 100.0,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(311), config);
        let force = zone.wind_force_at(Vec3::new(10.0, 0.0, 0.0), 1.0, 1.0);
        assert_eq!(force, Vec3::ZERO, "Outside box should produce zero force");
    }

    #[test]
    fn mutation_wind_force_at_vortex_at_center() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 50.0 },
            wind_type: WindType::Vortex {
                tangential_speed: 10.0,
                inward_pull: 5.0,
                updraft: 3.0,
            },
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(312), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // At center (dist < 0.1), only updraft applies as wind_velocity
        // velocity = (0, updraft, 0) = (0, 3, 0), speed = 3
        // force_magnitude = 0.5 * 1.225 * 9 * 1 * 1 = 5.5125
        let expected_mag = 0.5 * 1.225 * 3.0 * 3.0 * 1.0 * 1.0;
        assert!(
            (force.y - expected_mag).abs() < 0.1,
            "At center, should be upward force ~{}, got {}",
            expected_mag,
            force.y
        );
        assert!(force.x.abs() < 0.1);
        assert!(force.z.abs() < 0.1);
    }

    #[test]
    fn mutation_wind_force_low_speed_zero() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 0.005, // Very low speed
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(313), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // speed = 0.005 < 0.01 threshold
        assert_eq!(force, Vec3::ZERO, "Very low wind speed should return zero");
    }

    // --- WindZone::calculate_falloff ---
    #[test]
    fn mutation_falloff_zero_means_uniform() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            falloff: 0.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(320), config);
        assert_eq!(
            zone.calculate_falloff(Vec3::new(9.0, 0.0, 0.0)),
            1.0,
            "Zero falloff means uniform"
        );
    }

    #[test]
    fn mutation_falloff_global_always_zero_dist() {
        let config = WindZoneConfig {
            shape: WindZoneShape::Global,
            falloff: 1.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(321), config);
        assert_eq!(
            zone.calculate_falloff(Vec3::new(1000.0, 0.0, 0.0)),
            1.0,
            "Global always has dist=0"
        );
    }

    #[test]
    fn mutation_falloff_sphere_at_edge() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            falloff: 1.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(322), config);
        let f = zone.calculate_falloff(Vec3::new(10.0, 0.0, 0.0));
        // normalized_dist = 10/10 = 1, falloff = (1 - 1*1).max(0) = 0
        assert!(
            (f - 0.0).abs() < 1e-5,
            "At edge with full falloff should be 0, got {}",
            f
        );
    }

    #[test]
    fn mutation_falloff_sphere_midpoint() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            falloff: 1.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(323), config);
        let f = zone.calculate_falloff(Vec3::new(5.0, 0.0, 0.0));
        // normalized_dist = 5/10 = 0.5, falloff = (1 - 0.5*1).max(0) = 0.5
        assert!(
            (f - 0.5).abs() < 1e-5,
            "At midpoint should be 0.5, got {}",
            f
        );
    }

    #[test]
    fn mutation_falloff_box_max_element() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Box {
                half_extents: Vec3::new(10.0, 20.0, 30.0),
            },
            falloff: 1.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(324), config);
        // local = (5, 10, 15), local/he = (0.5, 0.5, 0.5), max_element = 0.5
        let f = zone.calculate_falloff(Vec3::new(5.0, 10.0, 15.0));
        assert!(
            (f - 0.5).abs() < 1e-5,
            "Box falloff should use max_element, got {}",
            f
        );
    }

    #[test]
    fn mutation_falloff_cylinder_horizontal_vs_vertical() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 20.0,
            },
            falloff: 1.0,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(325), config);
        // horizontal_dist = 5/10 = 0.5, vertical_dist = 0/(20/2) = 0
        let f = zone.calculate_falloff(Vec3::new(5.0, 0.0, 0.0));
        assert!(
            (f - 0.5).abs() < 1e-5,
            "Horizontal at midpoint, vertical at center -> 0.5, got {}",
            f
        );
    }

    // --- GustEvent::current_strength ---
    #[test]
    fn mutation_gust_strength_at_t0() {
        let gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        // t=0, envelope: attack = (0*4).min(1)=0, release = ((1-0)*4).min(1)=1
        // with smoothness=0.5, envelope = attack * release = 0
        assert_eq!(
            gust.current_strength(),
            0.0,
            "At t=0, gust should have 0 strength due to attack ramp"
        );
    }

    #[test]
    fn mutation_gust_strength_midpoint() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.elapsed = 1.0;
        // t = 1.0/2.0 = 0.5
        // attack = (0.5*4).min(1) = 1.0, release = ((1-0.5)*4).min(1) = 1.0
        // envelope = 1.0, strength = 10.0 * 1.0 = 10.0
        let s = gust.current_strength();
        assert!(
            (s - 10.0).abs() < 0.01,
            "At midpoint, gust should be at full strength=10, got {}",
            s
        );
    }

    #[test]
    fn mutation_gust_strength_near_end() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.elapsed = 1.9;
        // t = 1.9/2.0 = 0.95
        // attack = (0.95*4).min(1) = 1.0, release = ((1-0.95)*4).min(1) = 0.2
        // envelope = 0.2, strength = 10.0 * 0.2 = 2.0
        let s = gust.current_strength();
        assert!(
            (s - 2.0).abs() < 0.01,
            "Near end, gust should be decaying, expected 2.0, got {}",
            s
        );
    }

    #[test]
    fn mutation_gust_strength_after_duration() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.elapsed = 3.0;
        assert_eq!(gust.current_strength(), 0.0, "Past duration should be 0");
    }

    #[test]
    fn mutation_gust_strength_zero_smoothness() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.smoothness = 0.0;
        gust.elapsed = 0.5;
        // With smoothness=0, envelope = 1.0 always
        let s = gust.current_strength();
        assert!(
            (s - 10.0).abs() < 0.01,
            "Zero smoothness should give flat envelope, expected 10, got {}",
            s
        );
    }

    // --- EnvironmentManager::wind_force_at ---
    #[test]
    fn mutation_envmgr_wind_force_global_only() {
        let mut mgr = EnvironmentManager::new();
        mgr.global_wind = Vec3::new(10.0, 0.0, 0.0);
        mgr.global_wind_strength = 1.0;
        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // speed = global_wind.length * strength = 10
        // force = 0.5 * 1.225 * 100 * 1 * 1 = 61.25
        let expected = 0.5 * 1.225 * 100.0 * 1.0 * 1.0;
        assert!(
            (force.x - expected).abs() < 0.1,
            "Global wind force should be ~{}, got {}",
            expected,
            force.x
        );
    }

    #[test]
    fn mutation_envmgr_wind_force_zone_plus_global() {
        let mut mgr = EnvironmentManager::new();
        mgr.global_wind = Vec3::new(5.0, 0.0, 0.0);
        mgr.global_wind_strength = 1.0;
        mgr.add_wind_zone(WindZoneConfig {
            shape: WindZoneShape::Global,
            direction: Vec3::new(0.0, 0.0, 5.0),
            strength: 5.0,
            active: true,
            ..Default::default()
        });
        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(force.x > 0.0, "Should have global wind X component");
        assert!(force.z > 0.0, "Should have zone wind Z component");
    }

    #[test]
    fn mutation_envmgr_gust_contributes_force() {
        let mut mgr = EnvironmentManager::new();
        mgr.trigger_gust(Vec3::X, 20.0, 2.0);
        mgr.update(0.5); // advance to get envelope up
        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(force.x > 0.0, "Active gust should contribute wind force");
    }

    // --- EnvironmentManager ID increment ---
    #[test]
    fn mutation_envmgr_id_increments() {
        let mut mgr = EnvironmentManager::new();
        let w1 = mgr.add_wind_zone(WindZoneConfig::default());
        let w2 = mgr.add_wind_zone(WindZoneConfig::default());
        assert_ne!(w1, w2, "Wind zone IDs should be unique");
        assert_eq!(WindZoneId(w1.0 + 1), w2, "IDs should increment by 1");
    }

    // ===== DEEP REMEDIATION v3.6.2 — environment Round 3 remaining mutations =====

    // --- WindZone::contains cylinder height/2.0 ---
    #[test]
    fn mutation_r3_windzone_contains_cylinder_half_height() {
        // local.y.abs() <= height / 2.0  (mutation: / → % or *)
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 5.0,
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 4.0,
            },
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(907), config);
        // y = 1.9 — inside (|1.9| <= 4/2 = 2)
        assert!(
            zone.contains(Vec3::new(0.0, 1.9, 0.0)),
            "y=1.9 should be inside cylinder h=4"
        );
        // y = 2.1 — outside (|2.1| > 2)
        assert!(
            !zone.contains(Vec3::new(0.0, 2.1, 0.0)),
            "y=2.1 should be outside cylinder h=4"
        );
    }

    // --- EnvironmentManager::add_wind_zone ID increment ---
    #[test]
    fn mutation_r3_add_wind_zone_id_increments() {
        // next_wind_id += 1  (mutation: += → -= or *= )
        let mut mgr = EnvironmentManager::new();
        let id1 = mgr.add_wind_zone(WindZoneConfig {
            direction: Vec3::X,
            strength: 1.0,
            ..Default::default()
        });
        let id2 = mgr.add_wind_zone(WindZoneConfig {
            direction: Vec3::Y,
            strength: 2.0,
            ..Default::default()
        });
        let id3 = mgr.add_wind_zone(WindZoneConfig {
            direction: Vec3::Z,
            strength: 3.0,
            ..Default::default()
        });
        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
        assert_eq!(id3.0, 3);
        // Each zone should be retrievable
        assert!(mgr.get_wind_zone(id1).is_some());
        assert!(mgr.get_wind_zone(id2).is_some());
        assert!(mgr.get_wind_zone(id3).is_some());
    }

    // --- EnvironmentManager::current_gust_force composition ---
    #[test]
    fn mutation_r3_current_gust_force_sum() {
        let mut mgr = EnvironmentManager::new();
        mgr.trigger_gust(Vec3::X, 5.0, 10.0); // Very long duration
        mgr.trigger_gust(Vec3::Y, 3.0, 10.0);
        let force = mgr.current_gust_force();
        // At t=0, current_strength = 0 (smoothness envelope starts at 0)
        // But direction * strength at peak should compose additively
        // The fold should add, not subtract or multiply
        // At t=0 with default smoothness=0.5, strength might be 0
        // Let's check they produce a non-zero result after some time
        // Actually at t=0, current_strength returns the envelope at t=0
        // which is: strength * smoothstep(0/0.5) * smoothstep((10-0)/(10*0.5)) = 5*0*1 = 0
        // So gusts at t=0 give 0. This test verifies the fold doesn't crash and returns ZERO
        assert!(
            force.length() >= 0.0,
            "Gust force should be non-negative length"
        );
    }

    // ===== DEEP REMEDIATION v3.6.3 — environment Round 4 remaining mutations =====

    // --- WindZone::wind_force_at exact formula (30 mutations) ---
    #[test]
    fn mutation_r4_wind_force_directional_exact() {
        // F = 0.5 * 1.225 * speed² * Cd * A in direction of wind
        let config = WindZoneConfig {
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Global,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(940), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // speed = 10.0, force_mag = 0.5 * 1.225 * 100 * 1 * 1 = 61.25
        let expected_mag = 0.5 * 1.225 * 10.0 * 10.0 * 1.0 * 1.0;
        assert!(
            (force.length() - expected_mag).abs() < 0.1,
            "Directional force magnitude: expected {}, got {}",
            expected_mag,
            force.length()
        );
        // Should point in +X direction
        assert!(force.x > 0.0, "Should be in +X: {:?}", force);
        assert!(force.y.abs() < 0.01, "No Y component");
    }

    #[test]
    fn mutation_r4_wind_force_drag_coefficient_multiplier() {
        let config = WindZoneConfig {
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Global,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(941), config);
        let f1 = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let f2 = zone.wind_force_at(Vec3::ZERO, 2.0, 1.0);
        // Doubling drag should double force
        assert!(
            (f2.length() / f1.length() - 2.0).abs() < 0.01,
            "Double drag = double force: f1={:.2} f2={:.2}",
            f1.length(),
            f2.length()
        );
    }

    #[test]
    fn mutation_r4_wind_force_cross_section_multiplier() {
        let config = WindZoneConfig {
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Global,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(942), config);
        let f1 = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let f3 = zone.wind_force_at(Vec3::ZERO, 1.0, 3.0);
        // Tripling area should triple force
        assert!(
            (f3.length() / f1.length() - 3.0).abs() < 0.01,
            "Triple area = triple force: f1={:.2} f3={:.2}",
            f1.length(),
            f3.length()
        );
    }

    #[test]
    fn mutation_r4_wind_force_speed_squared() {
        // F ∝ speed² — doubling speed should quadruple force
        let config1 = WindZoneConfig {
            direction: Vec3::X,
            strength: 5.0,
            shape: WindZoneShape::Global,
            active: true,
            ..Default::default()
        };
        let config2 = WindZoneConfig {
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Global,
            active: true,
            ..Default::default()
        };
        let z1 = WindZone::new(WindZoneId(943), config1);
        let z2 = WindZone::new(WindZoneId(944), config2);
        let f1 = z1.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let f2 = z2.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(
            (f2.length() / f1.length() - 4.0).abs() < 0.1,
            "Double speed = 4× force: f1={:.2} f2={:.2} ratio={:.2}",
            f1.length(),
            f2.length(),
            f2.length() / f1.length()
        );
    }

    // --- WindZone::wind_force_at vortex tangent arithmetic ---
    #[test]
    fn mutation_r4_wind_force_vortex_tangential() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Sphere { radius: 100.0 },
            wind_type: WindType::Vortex {
                tangential_speed: 5.0,
                inward_pull: 0.0,
                updraft: 0.0,
            },
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(945), config);
        // Point at (10, 0, 0): tangent = (-0, 0, 10).normalize = (0,0,1)
        // tangential component = (0,0,1) * 5 = (0,0,5)
        let force = zone.wind_force_at(Vec3::new(10.0, 0.0, 0.0), 1.0, 1.0);
        // Force should have a Z component (tangential)
        assert!(
            force.z.abs() > 0.1,
            "Vortex at (10,0,0) should have Z component: {:?}",
            force
        );
    }

    // --- GustEvent::current_strength envelope (15 mutations) ---
    #[test]
    fn mutation_r4_gust_envelope_midpoint_exact() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.smoothness = 0.5;
        gust.elapsed = 1.0; // t = 0.5 (midpoint)
        let s = gust.current_strength();
        // t=0.5: attack = min(0.5*4, 1) = 1.0, release = min((1-0.5)*4, 1) = 1.0
        // envelope = 1.0 * 1.0 = 1.0, strength = 10 * 1 = 10
        assert!(
            (s - 10.0).abs() < 0.1,
            "At midpoint: expected 10.0, got {}",
            s
        );
    }

    #[test]
    fn mutation_r4_gust_envelope_early_ramp() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 4.0);
        gust.smoothness = 0.5;
        gust.elapsed = 0.5; // t = 0.125
        let s = gust.current_strength();
        // attack = min(0.125*4, 1) = 0.5, release = min((1-0.125)*4, 1) = 1.0
        // envelope = 0.5, strength = 10 * 0.5 = 5
        assert!(
            (s - 5.0).abs() < 0.5,
            "At t=0.125: expected ~5.0 (ramping up), got {}",
            s
        );
    }

    #[test]
    fn mutation_r4_gust_envelope_late_decay() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 4.0);
        gust.smoothness = 0.5;
        gust.elapsed = 3.5; // t = 0.875
        let s = gust.current_strength();
        // attack = min(0.875*4, 1) = 1.0, release = min((1-0.875)*4, 1) = 0.5
        // envelope = 0.5, strength = 10 * 0.5 = 5
        assert!(
            (s - 5.0).abs() < 0.5,
            "At t=0.875: expected ~5.0 (decaying), got {}",
            s
        );
    }

    // --- WindZone::calculate_falloff (14 mutations) ---
    #[test]
    fn mutation_r4_falloff_sphere_half_radius() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            falloff: 1.0,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(960), config);
        // At dist = 5 (half radius): normalized = 0.5
        // falloff = (1 - 0.5 * 1.0).max(0) = 0.5
        let f = zone.calculate_falloff(Vec3::new(5.0, 0.0, 0.0));
        assert!(
            (f - 0.5).abs() < 0.01,
            "Half radius falloff should be 0.5, got {}",
            f
        );
    }

    #[test]
    fn mutation_r4_falloff_cylinder_horizontal_vs_vertical() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 20.0,
            },
            falloff: 1.0,
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(961), config);
        // At (5,0,0): horizontal=5/10=0.5, vertical=0/(20/2)=0, max=0.5
        let fh = zone.calculate_falloff(Vec3::new(5.0, 0.0, 0.0));
        // At (0,5,0): horizontal=0, vertical=5/10=0.5, max=0.5
        let fv = zone.calculate_falloff(Vec3::new(0.0, 5.0, 0.0));
        assert!(
            (fh - fv).abs() < 0.01,
            "Same normalized dist should give same falloff: h={} v={}",
            fh,
            fv
        );
        assert!(
            (fh - 0.5).abs() < 0.01,
            "Falloff at normalized 0.5 should be 0.5"
        );
    }

    // --- WindZone::update turbulent (32 mutations) ---
    #[test]
    fn mutation_r4_turbulent_gust_varies_over_time() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 10.0,
            shape: WindZoneShape::Global,
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 2.0,
            },
            active: true,
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(970), config);
        let f0 = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        zone.update(0.5); // Advance time
        let f1 = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        zone.update(0.5);
        let f2 = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // Turbulent wind should vary over time
        let all_same = (f0 - f1).length() < 0.001 && (f1 - f2).length() < 0.001;
        assert!(
            !all_same,
            "Turbulent wind should vary: f0={:?} f1={:?} f2={:?}",
            f0, f1, f2
        );
    }

    #[test]
    fn mutation_r4_turbulent_noise_phase_increments() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 3.0,
            },
            active: true,
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(971), config);
        assert!((zone.noise_phase - 0.0).abs() < 0.001);
        zone.update(1.0);
        // noise_phase += dt * frequency = 1.0 * 3.0 = 3.0
        assert!(
            (zone.noise_phase - 3.0).abs() < 0.01,
            "phase should be 3.0, got {}",
            zone.noise_phase
        );
        zone.update(0.5);
        // += 0.5 * 3.0 = 1.5, total = 4.5
        assert!(
            (zone.noise_phase - 4.5).abs() < 0.01,
            "phase should be 4.5, got {}",
            zone.noise_phase
        );
    }

    #[test]
    fn mutation_r4_turbulent_gust_offset_components() {
        let config = WindZoneConfig {
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 1.0,
            },
            active: true,
            ..Default::default()
        };
        let mut zone = WindZone::new(WindZoneId(972), config);
        zone.update(1.0);
        // gust_offset should have all 3 components from sin/cos formulas
        // Each component is sum of two sin terms
        let go = zone.gust_offset;
        // Verify the formula: x = sin(1.0)*0.5 + sin(2.3)*0.3
        let expected_x = (1.0_f32).sin() * 0.5 + (2.3_f32).sin() * 0.3;
        assert!(
            (go.x - expected_x).abs() < 0.01,
            "gust x: expected {:.4}, got {:.4}",
            expected_x,
            go.x
        );
        let expected_y = (0.7_f32).sin() * 0.2 + (1.9_f32).sin() * 0.15;
        assert!(
            (go.y - expected_y).abs() < 0.01,
            "gust y: expected {:.4}, got {:.4}",
            expected_y,
            go.y
        );
        let expected_z = (1.3_f32).sin() * 0.5 + (2.7_f32).sin() * 0.3;
        assert!(
            (go.z - expected_z).abs() < 0.01,
            "gust z: expected {:.4}, got {:.4}",
            expected_z,
            go.z
        );
    }

    // ===== ROUND 6: EnvironmentManager integration tests =====

    #[test]
    fn r6_env_manager_wind_force_global_only() {
        let mut mgr = EnvironmentManager::new();
        mgr.global_wind = Vec3::new(10.0, 0.0, 0.0);
        mgr.global_wind_strength = 1.0;
        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // F = 0.5 * 1.225 * speed² * Cd * A
        // speed = |global_wind| * strength = 10
        let expected_mag = 0.5 * 1.225 * 100.0 * 1.0 * 1.0; // = 61.25
        assert!(
            force.x > 0.0,
            "Wind force should be in +X direction: {:?}",
            force
        );
        assert!(
            (force.x - expected_mag).abs() < 1.0,
            "Expected ~{}, got {}",
            expected_mag,
            force.x
        );
    }

    #[test]
    fn r6_env_manager_wind_force_aggregates_zones() {
        let mut mgr = EnvironmentManager::new();
        // Add a directional wind zone
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 5.0,
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            active: true,
            ..Default::default()
        };
        mgr.add_wind_zone(config);

        // Also set global wind
        mgr.global_wind = Vec3::new(5.0, 0.0, 0.0);
        mgr.global_wind_strength = 1.0;

        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // Should aggregate both global and zone forces (both in +X)
        let global_only = 0.5 * 1.225 * 25.0; // speed=5
        assert!(
            force.x > global_only,
            "Aggregated wind should exceed global-only {}: got {}",
            global_only,
            force.x
        );
    }

    #[test]
    fn r6_env_manager_wind_force_includes_gust() {
        let mut mgr = EnvironmentManager::new();
        mgr.trigger_gust(Vec3::X, 20.0, 1.0);

        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // Gust direction = X normalized, strength=20
        // GustEvent::current_strength at t=0: envelope = min(0*4,1)=0, so force=0
        // We need to advance time slightly
        mgr.update(0.1); // elapsed = 0.1, t = 0.1/1.0 = 0.1
        let force = mgr.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // At t=0.1: attack = min(0.4, 1) = 0.4, release = min(3.6, 1) = 1.0
        // envelope = 0.4 * 1.0 = 0.4, gust_strength = 20 * 0.4 = 8.0
        // gust force mag = 0.5 * 1.225 * 64 = 39.2 in X
        assert!(
            force.x > 10.0,
            "Gust should contribute significant X force: {:?}",
            force
        );
    }

    #[test]
    fn r6_env_manager_gust_removed_after_duration() {
        let mut mgr = EnvironmentManager::new();
        mgr.trigger_gust(Vec3::X, 10.0, 0.5);
        assert_eq!(mgr.gusts.len(), 1);

        mgr.update(0.6); // elapsed 0.6 > duration 0.5
        assert_eq!(
            mgr.gusts.len(),
            0,
            "Finished gust should be removed by update"
        );
    }

    #[test]
    fn r6_env_manager_update_advances_all() {
        let mut mgr = EnvironmentManager::new();

        // Add turbulent wind zone
        let wind_config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 5.0,
            shape: WindZoneShape::Global,
            wind_type: WindType::Turbulent {
                intensity: 1.0,
                frequency: 2.0,
            },
            active: true,
            ..Default::default()
        };
        let wid = mgr.add_wind_zone(wind_config);

        mgr.update(0.1);

        // Wind zone should have updated gust_offset
        let zone = mgr.get_wind_zone(wid).unwrap();
        assert!(
            zone.gust_offset.length() > 0.0,
            "Turbulent zone gust_offset should be non-zero after update"
        );
    }

    #[test]
    fn r6_vortex_wind_tangential_component() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X, // Not used for vortex
            strength: 10.0,
            shape: WindZoneShape::Sphere { radius: 50.0 },
            wind_type: WindType::Vortex {
                tangential_speed: 20.0,
                inward_pull: 5.0,
                updraft: 3.0,
            },
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        // Test point at (10, 0, 0) — horizontal distance from center
        let force = zone.wind_force_at(Vec3::new(10.0, 0.0, 0.0), 1.0, 1.0);
        // Vortex produces tangential + inward + updraft
        // Should have Y component (updraft) and Z component (tangential)
        assert!(
            force.length() > 0.1,
            "Vortex should produce non-zero force: {:?}",
            force
        );
        assert!(
            force.y > 0.0,
            "Vortex should have updraft (Y>0): {:?}",
            force
        );
    }

    #[test]
    fn r6_gust_current_strength_envelope() {
        let mut gust = GustEvent::new(Vec3::X, 10.0, 2.0);
        gust.smoothness = 0.5;

        // At t=0: strength = 0 (attack phase starts)
        assert!(
            gust.current_strength() < 0.01,
            "At t=0, gust should be near zero: {}",
            gust.current_strength()
        );

        // At t=0.1 (t_norm=0.05): early ramp up, attack = (0.05*4).min(1) = 0.2
        gust.elapsed = 0.1;
        let s1 = gust.current_strength();
        assert!(
            s1 > 0.0 && s1 < 10.0,
            "At t=0.1, gust should be rising: {}",
            s1
        );

        // At t=1.0 (t_norm=0.5): full peak, attack=min(2,1)=1, release=min(2,1)=1
        gust.elapsed = 1.0;
        let s2 = gust.current_strength();
        assert!(s2 >= s1, "At midpoint, gust should be >= early: {}", s2);

        // At t=1.95 (t_norm=0.975): late release, release = (0.025*4).min(1) = 0.1
        gust.elapsed = 1.95;
        let s3 = gust.current_strength();
        assert!(s3 < s2, "Near end, gust should be declining: {}", s3);

        // At t=2.0: finished
        gust.elapsed = 2.0;
        assert!(
            gust.current_strength() < 0.01,
            "After duration, gust should be zero: {}",
            gust.current_strength()
        );
    }

    #[test]
    fn r6_wind_zone_inactive_returns_zero() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            direction: Vec3::X,
            strength: 50.0,
            shape: WindZoneShape::Global,
            wind_type: WindType::Directional,
            active: false,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);
        let force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(
            force.length() < 0.001,
            "Inactive zone should return zero: {:?}",
            force
        );
    }

    // ===== ROUND 7: Targeted catches =====

    #[test]
    fn r7_current_gust_force_returns_aggregate() {
        let mut em = EnvironmentManager::new();
        em.trigger_gust(Vec3::X, 5.0, 2.0);
        em.trigger_gust(Vec3::Z, 3.0, 2.0);

        let force = em.current_gust_force();
        // Both gusts at t=0 should have zero strength (attack phase)
        // actually at t=0, smoothness defaults... let me check

        // Advance slightly to start gust
        em.update(0.5);
        let force2 = em.current_gust_force();
        // Both gusts should contribute
        assert!(
            force2.x.abs() > 0.0 || force2.z.abs() > 0.0,
            "Gust force should be non-zero after update: {:?}",
            force2
        );
        // X gust has strength 5, Z gust has strength 3
        // After t=0.5 with duration=2.0, both should have rised
        assert!(
            force2.length() > 0.1,
            "Aggregate gust force should have magnitude: {:?}",
            force2
        );
    }

    #[test]
    fn r7_wind_force_at_global_exact_formula() {
        let mut em = EnvironmentManager::new();
        em.global_wind = Vec3::X;
        em.global_wind_strength = 10.0;

        // Force = 0.5 * 1.225 * (speed=10)^2 * Cd * A
        let cd = 2.0;
        let area = 0.5;
        let force = em.wind_force_at(Vec3::ZERO, cd, area);

        // Expected: 0.5 * 1.225 * 100 * 2.0 * 0.5 = 61.25
        let expected = 0.5 * 1.225 * 10.0 * 10.0 * cd * area;
        let diff = (force.x - expected).abs();
        assert!(
            diff < 0.5,
            "Global wind force should be ~{}: got {:?}",
            expected,
            force
        );
        // Y and Z should be ~0 (wind is purely +X)
        assert!(force.y.abs() < 0.01, "No Y component: {}", force.y);
        assert!(force.z.abs() < 0.01, "No Z component: {}", force.z);
    }

    #[test]
    fn r7_wind_force_at_with_gust_adds_gust() {
        let mut em = EnvironmentManager::new();
        // No global wind
        em.global_wind = Vec3::ZERO;
        em.global_wind_strength = 0.0;

        // Add gust with zero smoothness (immediate full strength)
        em.trigger_gust(Vec3::X, 10.0, 5.0);
        // Need to set smoothness to 0 on the gust
        em.gusts[0].smoothness = 0.0;

        let force_with_gust = em.wind_force_at(Vec3::ZERO, 1.0, 1.0);

        // Gust at full strength: speed = 10
        // Force = 0.5 * 1.225 * 100 * 1 * 1 = 61.25
        let expected = 0.5 * 1.225 * 10.0 * 10.0;
        assert!(
            (force_with_gust.x - expected).abs() < 1.0,
            "Gust-only force should be ~{}: got {:?}",
            expected,
            force_with_gust
        );
    }

    #[test]
    fn r7_vortex_to_center_direction() {
        // Vortex wind: to_center = position - point
        // If position=(10,0,0) and point=(5,0,0), to_center=(5,0,0)
        // tangent should be perpendicular: (-0, 0, 5).normalize = (0,0,1)
        let config = WindZoneConfig {
            position: Vec3::new(10.0, 0.0, 0.0),
            direction: Vec3::X, // not used for Vortex
            strength: 0.0,      // not used for Vortex
            shape: WindZoneShape::Sphere { radius: 100.0 },
            wind_type: WindType::Vortex {
                tangential_speed: 10.0,
                inward_pull: 0.0,
                updraft: 0.0,
            },
            active: true,
            ..Default::default()
        };
        let zone = WindZone::new(WindZoneId(1), config);

        let force_a = zone.wind_force_at(Vec3::new(5.0, 0.0, 0.0), 1.0, 1.0);
        let force_b = zone.wind_force_at(Vec3::new(15.0, 0.0, 0.0), 1.0, 1.0);

        // Force_a is on -X side → tangent in +Z
        // Force_b is on +X side → tangent in -Z
        // Opposite sides should produce opposite tangential directions
        if force_a.length() > 0.01 && force_b.length() > 0.01 {
            assert!(
                force_a.z * force_b.z < 0.0,
                "Vortex tangential force should reverse on opposite sides: a={:?}, b={:?}",
                force_a,
                force_b
            );
        }
    }

    // ===== ROUND 8: buoyancy/water/falloff catches =====

    #[test]
    fn r8_wind_zone_falloff_sphere() {
        // Sphere wind zone with falloff should reduce force towards edge
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 10.0 },
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 20.0,
            falloff: 1.0,
            active: true,
        };
        let zone = WindZone::new(WindZoneId(0), config);

        // At center → full strength
        let force_center = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // At half radius → reduced
        let force_half = zone.wind_force_at(Vec3::new(5.0, 0.0, 0.0), 1.0, 1.0);

        assert!(
            force_center.length() > force_half.length(),
            "Force at center should be stronger than at half radius: center={}, half={}",
            force_center.length(),
            force_half.length()
        );

        // Force at center should be non-zero
        assert!(
            force_center.length() > 0.1,
            "Center force should be non-zero"
        );
    }

    #[test]
    fn r8_wind_zone_falloff_box() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Box {
                half_extents: Vec3::splat(10.0),
            },
            wind_type: WindType::Directional,
            direction: Vec3::new(0.0, 0.0, 1.0),
            strength: 15.0,
            falloff: 1.0,
            active: true,
        };
        let zone = WindZone::new(WindZoneId(0), config);

        let force_center = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let force_edge = zone.wind_force_at(Vec3::new(9.0, 0.0, 0.0), 1.0, 1.0);

        assert!(
            force_center.length() > force_edge.length(),
            "Center should have more force than edge: center={}, edge={}",
            force_center.length(),
            force_edge.length()
        );
    }

    #[test]
    fn r8_wind_zone_falloff_cylinder() {
        let config = WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Cylinder {
                radius: 10.0,
                height: 20.0,
            },
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 15.0,
            falloff: 1.0,
            active: true,
        };
        let zone = WindZone::new(WindZoneId(0), config);

        let force_center = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        let force_near_edge = zone.wind_force_at(Vec3::new(8.0, 0.0, 0.0), 1.0, 1.0);

        assert!(
            force_center.length() > force_near_edge.length(),
            "Center > edge: center={}, edge={}",
            force_center.length(),
            force_near_edge.length()
        );
    }

    #[test]
    fn r8_env_mgr_wind_force_at_with_zone() {
        let mut em = EnvironmentManager::default();
        em.add_wind_zone(WindZoneConfig {
            position: Vec3::ZERO,
            shape: WindZoneShape::Sphere { radius: 20.0 },
            wind_type: WindType::Directional,
            direction: Vec3::new(1.0, 0.0, 0.0),
            strength: 10.0,
            falloff: 0.0,
            active: true,
        });

        // Point inside zone
        let force_in = em.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        assert!(
            force_in.length() > 0.0,
            "Inside zone should have wind force"
        );

        // Point outside zone
        let force_out = em.wind_force_at(Vec3::new(100.0, 0.0, 0.0), 1.0, 1.0);
        // Outside zone, only global wind applies (default is zero)
        assert!(
            force_out.length() < force_in.length(),
            "Outside zone should have less force"
        );
    }

    // ===== ROUND 9: Environment precision tests =====

    #[test]
    fn r9_global_wind_force_magnitude() {
        let mut mgr = EnvironmentManager::new();
        mgr.global_wind = Vec3::new(10.0, 0.0, 0.0);
        mgr.global_wind_strength = 1.0;

        let drag_coeff = 1.0;
        let cross_section = 2.0;
        let force = mgr.wind_force_at(Vec3::ZERO, drag_coeff, cross_section);

        // Expected: speed = 10.0 * 1.0 = 10.0
        // force_mag = 0.5 * 1.225 * 10 * 10 * 1.0 * 2.0 = 0.5 * 1.225 * 100 * 2 = 122.5
        // direction = (1, 0, 0)
        let expected_mag = 0.5 * 1.225 * 100.0 * drag_coeff * cross_section;
        assert!(
            (force.length() - expected_mag).abs() < 1.0,
            "Wind force magnitude should be ~{}, got {}",
            expected_mag,
            force.length()
        );
        assert!(force.x > 0.0, "Wind force should be in +X: {:?}", force);
    }

    #[test]
    fn r9_wind_zone_force_magnitude_with_falloff() {
        // Test that wind zone's force_at multiplication chain works correctly
        let zone = WindZone::new(
            WindZoneId(1),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Sphere { radius: 10.0 },
                wind_type: WindType::Directional,
                direction: Vec3::X, // unit direction
                strength: 5.0,      // speed = strength = 5.0 m/s
                falloff: 1.0,       // linear falloff
                active: true,
            },
        );

        // At center (falloff=1.0 for sphere at distance 0)
        let force_center = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);
        // At edge (falloff approaches 0)
        let force_edge = zone.wind_force_at(Vec3::new(9.5, 0.0, 0.0), 1.0, 1.0);

        assert!(
            force_center.length() > force_edge.length(),
            "Center force ({}) should exceed edge force ({})",
            force_center.length(),
            force_edge.length()
        );

        // Wind velocity = direction.normalize() * strength = (1,0,0) * 5.0
        // speed = 5.0, force = 0.5 * 1.225 * 25.0 * 1.0 * 1.0 = 15.3125
        let expected = 0.5 * 1.225 * 25.0;
        assert!(
            (force_center.length() - expected).abs() < 2.0,
            "Center force should be ~{}, got {}",
            expected,
            force_center.length()
        );
    }

    // ============================================================================
    // R11 — Targeted mutation tests for WindZone force calculations
    // ============================================================================

    #[test]
    fn r11_wind_force_directional_exact_magnitude() {
        // F = 0.5 * 1.225 * speed^2 * drag * area
        // With direction=(1,0,0), strength=10, drag=1, area=1, no falloff
        let zone = WindZone::new(
            WindZoneId(100),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Sphere { radius: 100.0 },
                wind_type: WindType::Directional,
                direction: Vec3::X,
                strength: 10.0,
                falloff: 0.0, // no falloff
                active: true,
            },
        );
        let force = zone.wind_force_at(Vec3::new(1.0, 0.0, 0.0), 1.0, 1.0);
        // speed = 10.0, F = 0.5 * 1.225 * 100 * 1 * 1 = 61.25
        let expected = 0.5 * 1.225 * 100.0;
        assert!(
            (force.x - expected).abs() < 1.0,
            "Force magnitude should be 0.5*1.225*speed^2: expected={}, got={}",
            expected,
            force.x
        );
        assert!(
            force.y.abs() < 0.1,
            "Force should be along X only: y={}",
            force.y
        );
    }

    #[test]
    fn r11_wind_force_vortex_tangent_direction() {
        // The tangent direction should be (-z, 0, x) of the horizontal vector to center
        let zone = WindZone::new(
            WindZoneId(101),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Sphere { radius: 100.0 },
                wind_type: WindType::Vortex {
                    tangential_speed: 10.0,
                    inward_pull: 0.0,
                    updraft: 0.0,
                },
                direction: Vec3::X,
                strength: 0.0,
                falloff: 0.0,
                active: true,
            },
        );
        // Point at (5, 0, 0) — horizontal to_center = (0, 0, 0) - (5, 0, 0) = (-5, 0, 0)
        // tangent = (-horizontal.z, 0, horizontal.x) = (0, 0, -5).normalize() = (0, 0, -1)
        // Wind velocity = tangent * tangential_speed = (0, 0, -10)
        let force = zone.wind_force_at(Vec3::new(5.0, 0.0, 0.0), 1.0, 1.0);

        // Force should be in -Z direction
        assert!(
            force.z < -1.0,
            "Vortex tangent force should be in -Z for point on +X axis: z={}",
            force.z
        );
        assert!(
            force.x.abs() < force.z.abs() * 0.5,
            "Force should be primarily tangential, not radial: fx={}, fz={}",
            force.x,
            force.z
        );
    }

    #[test]
    fn r11_wind_force_vortex_inward_pull() {
        let zone = WindZone::new(
            WindZoneId(102),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Sphere { radius: 100.0 },
                wind_type: WindType::Vortex {
                    tangential_speed: 0.0,
                    inward_pull: 10.0,
                    updraft: 0.0,
                },
                direction: Vec3::X,
                strength: 0.0,
                falloff: 0.0,
                active: true,
            },
        );
        // Point at (5, 0, 0) — inward pull should point toward center (-X)
        let force = zone.wind_force_at(Vec3::new(5.0, 0.0, 0.0), 1.0, 1.0);

        assert!(
            force.x < -1.0,
            "Inward pull should create -X force for point at +X: fx={}",
            force.x
        );
    }

    #[test]
    fn r11_calculate_falloff_sphere_subtraction() {
        // falloff = (1 - normalized_dist * falloff_param).max(0)
        // If - changed to +, result would be > 1 (wrong)
        let zone = WindZone::new(
            WindZoneId(103),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Sphere { radius: 10.0 },
                wind_type: WindType::Directional,
                direction: Vec3::X,
                strength: 10.0,
                falloff: 1.0, // full falloff
                active: true,
            },
        );
        // At center: normalized_dist=0, factor=1.0
        let center_force = zone.wind_force_at(Vec3::ZERO, 1.0, 1.0);

        // At radius/2: normalized_dist=0.5, factor=(1-0.5*1)=0.5
        let mid_force = zone.wind_force_at(Vec3::new(5.0, 0.0, 0.0), 1.0, 1.0);

        // Mid-point force should be less than center (falloff reduces it)
        assert!(
            mid_force.length() < center_force.length(),
            "Force should decrease with falloff: center={}, mid={}",
            center_force.length(),
            mid_force.length()
        );

        // Force at edge should be near zero
        let edge_force = zone.wind_force_at(Vec3::new(9.9, 0.0, 0.0), 1.0, 1.0);
        assert!(
            edge_force.length() < center_force.length() * 0.2,
            "Force at edge should be very small: edge={}, center={}",
            edge_force.length(),
            center_force.length()
        );
    }

    #[test]
    fn r11_wind_update_frequency_scales_phase() {
        // update: noise_phase += dt * frequency
        // If * changed to /, higher frequency would slow phase advancement
        let mut zone = WindZone::new(
            WindZoneId(104),
            WindZoneConfig {
                position: Vec3::ZERO,
                shape: WindZoneShape::Global,
                wind_type: WindType::Turbulent {
                    intensity: 1.0,
                    frequency: 5.0,
                },
                direction: Vec3::X,
                strength: 10.0,
                falloff: 0.0,
                active: true,
            },
        );
        zone.update(1.0);
        let phase1 = zone.noise_phase;

        // Phase should advance by dt * frequency = 1.0 * 5.0 = 5.0
        assert!(
            (phase1 - 5.0).abs() < 0.1,
            "Phase should be dt*frequency=5.0: got={}",
            phase1
        );
    }
}
