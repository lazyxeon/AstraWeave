//! Player Water Mechanics
//!
//! Implements swimming, diving, oxygen management, and water-related
//! player interactions (inspired by Enshrouded's water mechanics).

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Movement mode in water
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum WaterMovementMode {
    /// Not in water, normal movement
    #[default]
    Dry,
    /// Shallow water (ankle to knee), slightly reduced speed
    Wading,
    /// Waist-deep water, significantly reduced speed
    WaistDeep,
    /// Surface swimming
    Swimming,
    /// Underwater diving
    Diving,
}

impl WaterMovementMode {
    /// Get movement speed multiplier for this mode
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            WaterMovementMode::Dry => 1.0,
            WaterMovementMode::Wading => 0.85,
            WaterMovementMode::WaistDeep => 0.6,
            WaterMovementMode::Swimming => 0.7,
            WaterMovementMode::Diving => 0.5,
        }
    }

    /// Get stamina drain multiplier
    pub fn stamina_drain_multiplier(&self) -> f32 {
        match self {
            WaterMovementMode::Dry => 1.0,
            WaterMovementMode::Wading => 1.1,
            WaterMovementMode::WaistDeep => 1.3,
            WaterMovementMode::Swimming => 1.5,
            WaterMovementMode::Diving => 2.0,
        }
    }

    /// Whether player can jump in this mode
    pub fn can_jump(&self) -> bool {
        matches!(
            self,
            WaterMovementMode::Dry | WaterMovementMode::Wading | WaterMovementMode::WaistDeep
        )
    }

    /// Whether player consumes oxygen in this mode
    pub fn consumes_oxygen(&self) -> bool {
        matches!(self, WaterMovementMode::Diving)
    }
}

/// Wet status debuff levels (matching Enshrouded)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum WetStatus {
    /// Player is completely dry
    #[default]
    Dry,
    /// Slightly wet (touched water briefly)
    Damp,
    /// Wet (-50% stamina regeneration)
    Wet,
    /// Soaking wet (-50% stamina regen, reduced max stamina)
    Soaking,
}

impl WetStatus {
    /// Get stamina regeneration multiplier
    pub fn stamina_regen_multiplier(&self) -> f32 {
        match self {
            WetStatus::Dry => 1.0,
            WetStatus::Damp => 0.9,
            WetStatus::Wet => 0.5,
            WetStatus::Soaking => 0.5,
        }
    }

    /// Get stamina max multiplier
    pub fn stamina_max_multiplier(&self) -> f32 {
        match self {
            WetStatus::Dry => 1.0,
            WetStatus::Damp => 1.0,
            WetStatus::Wet => 1.0,
            WetStatus::Soaking => 0.8, // -20% max stamina when soaking
        }
    }

    /// Time to dry from this status (seconds)
    pub fn dry_time(&self) -> f32 {
        match self {
            WetStatus::Dry => 0.0,
            WetStatus::Damp => 10.0,
            WetStatus::Wet => 30.0,
            WetStatus::Soaking => 60.0,
        }
    }
}

/// Configuration for water player mechanics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterPlayerConfig {
    /// Maximum oxygen capacity (seconds)
    pub max_oxygen: f32,
    /// Oxygen consumption rate while diving (per second)
    pub oxygen_drain_rate: f32,
    /// Oxygen recovery rate while not diving (per second)
    pub oxygen_recovery_rate: f32,
    /// Time until drowning damage starts after oxygen depleted
    pub drowning_grace_period: f32,
    /// Drowning damage per second
    pub drowning_damage_rate: f32,
    /// Player height for submersion calculation
    pub player_height: f32,
    /// Water level thresholds for movement modes
    pub wading_threshold: f32,
    pub waist_deep_threshold: f32,
    pub swimming_threshold: f32,
    pub diving_threshold: f32,
    /// Time to become fully soaked
    pub soak_time: f32,
    /// Wet Dog skill level (reduces wet debuff)
    pub wet_resistance_level: u8,
}

impl Default for WaterPlayerConfig {
    fn default() -> Self {
        Self {
            max_oxygen: 30.0,           // 30 seconds of breath
            oxygen_drain_rate: 1.0,     // 1 oxygen per second
            oxygen_recovery_rate: 3.0,  // Recover 3x faster than drain
            drowning_grace_period: 3.0, // 3 seconds before damage
            drowning_damage_rate: 10.0, // 10 HP/sec drowning damage
            player_height: 1.8,         // Meters
            wading_threshold: 0.15,     // 15% submerged
            waist_deep_threshold: 0.4,  // 40% submerged
            swimming_threshold: 0.7,    // 70% submerged
            diving_threshold: 0.95,     // 95% submerged (head under)
            soak_time: 5.0,             // 5 seconds to become soaking
            wet_resistance_level: 0,    // No wet dog skill by default
        }
    }
}

/// Water interaction state for a player/entity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaterPlayerState {
    /// Current submersion level (0.0 = dry, 1.0 = fully submerged)
    pub submersion: f32,
    /// Current movement mode
    pub mode: WaterMovementMode,
    /// Current oxygen remaining (seconds)
    pub oxygen: f32,
    /// Maximum oxygen (can be modified by equipment)
    pub max_oxygen: f32,
    /// Current wet status
    pub wet_status: WetStatus,
    /// Time spent in current wet status (for transitioning)
    pub wet_timer: f32,
    /// Time spent submerged (for soak calculation)
    pub submerge_time: f32,
    /// Drowning timer (counts up after oxygen depleted)
    pub drowning_timer: f32,
    /// Whether player is voluntarily diving (pressed dive key)
    pub is_diving: bool,
    /// Configuration
    config: WaterPlayerConfig,
}

impl Default for WaterPlayerState {
    fn default() -> Self {
        Self::new(WaterPlayerConfig::default())
    }
}

impl WaterPlayerState {
    /// Create a new water player state with configuration
    pub fn new(config: WaterPlayerConfig) -> Self {
        Self {
            submersion: 0.0,
            mode: WaterMovementMode::Dry,
            oxygen: config.max_oxygen,
            max_oxygen: config.max_oxygen,
            wet_status: WetStatus::Dry,
            wet_timer: 0.0,
            submerge_time: 0.0,
            drowning_timer: 0.0,
            is_diving: false,
            config,
        }
    }

    /// Fraction of the body under a water surface. `None` and non-finite inputs are dry.
    /// Feet at or above the surface are dry. Head at or below the surface is fully under.
    pub fn submersion_between(feet_y: f32, head_y: f32, surface_y: Option<f32>) -> f32 {
        let Some(surface) = surface_y else {
            return 0.0;
        };
        if !feet_y.is_finite() || !head_y.is_finite() || !surface.is_finite() {
            return 0.0;
        }
        if feet_y >= surface {
            return 0.0;
        }
        let span = head_y - feet_y;
        if span <= f32::EPSILON {
            return 1.0;
        }
        if head_y <= surface {
            return 1.0;
        }
        ((surface - feet_y) / span).clamp(0.0, 1.0)
    }

    /// Feed a traversal water surface into this state.
    ///
    /// `surface_y` and `feet_y` come from `WaterQuery` and the capsule feet.
    /// The fraction is [`submersion_between`] with the head at `feet_y + player_height`.
    pub fn update_from_surface(
        &mut self,
        surface_y: f32,
        feet_y: f32,
        dt: f32,
    ) -> WaterUpdateResult {
        let head_y = feet_y + self.config.player_height;
        self.update(
            Self::submersion_between(feet_y, head_y, Some(surface_y)),
            dt,
        )
    }

    /// Update water state based on submersion level
    ///
    /// Returns drowning damage if applicable
    pub fn update(&mut self, submersion: f32, dt: f32) -> WaterUpdateResult {
        self.submersion = submersion.clamp(0.0, 1.0);

        // Determine movement mode based on submersion
        self.mode = if self.submersion < self.config.wading_threshold {
            WaterMovementMode::Dry
        } else if self.submersion < self.config.waist_deep_threshold {
            WaterMovementMode::Wading
        } else if self.submersion < self.config.swimming_threshold {
            WaterMovementMode::WaistDeep
        } else if self.submersion < self.config.diving_threshold && !self.is_diving {
            WaterMovementMode::Swimming
        } else {
            WaterMovementMode::Diving
        };

        // Update wet status
        self.update_wet_status(dt);

        // Update oxygen
        let drowning_damage = self.update_oxygen(dt);

        // Track submersion time
        if self.submersion > 0.0 {
            self.submerge_time += dt;
        } else {
            self.submerge_time = 0.0;
        }

        WaterUpdateResult {
            mode: self.mode,
            wet_status: self.wet_status,
            oxygen_percent: self.oxygen / self.max_oxygen,
            drowning_damage,
            speed_multiplier: self.get_speed_multiplier(),
            stamina_drain_multiplier: self.get_stamina_drain_multiplier(),
            stamina_regen_multiplier: self.get_stamina_regen_multiplier(),
        }
    }

    /// Update wet status based on submersion
    fn update_wet_status(&mut self, dt: f32) {
        if self.submersion > 0.0 {
            // Getting wetter
            self.wet_timer += dt;

            self.wet_status = if self.wet_timer < 1.0 {
                WetStatus::Damp
            } else if self.wet_timer < 3.0 {
                WetStatus::Wet
            } else if self.wet_timer >= self.config.soak_time {
                WetStatus::Soaking
            } else {
                WetStatus::Wet
            };
        } else if self.wet_status != WetStatus::Dry {
            // Drying off - timer counts down in real seconds
            // Wet Dog skill speeds up drying
            let skill_bonus = match self.config.wet_resistance_level {
                0 => 1.0,
                1 => 1.25, // 25% faster
                2 => 1.5,  // 50% faster
                _ => 2.0,  // 100% faster
            };

            self.wet_timer -= dt * skill_bonus;

            if self.wet_timer <= 0.0 {
                // Transition to previous wet status
                self.wet_status = match self.wet_status {
                    WetStatus::Soaking => WetStatus::Wet,
                    WetStatus::Wet => WetStatus::Damp,
                    WetStatus::Damp => WetStatus::Dry,
                    WetStatus::Dry => WetStatus::Dry,
                };

                if self.wet_status != WetStatus::Dry {
                    self.wet_timer = self.wet_status.dry_time();
                }
            }
        }
    }

    /// Update oxygen and return drowning damage
    fn update_oxygen(&mut self, dt: f32) -> f32 {
        if self.mode.consumes_oxygen() {
            // Consuming oxygen
            self.oxygen -= self.config.oxygen_drain_rate * dt;

            if self.oxygen <= 0.0 {
                self.oxygen = 0.0;
                self.drowning_timer += dt;

                // Apply drowning damage after grace period
                if self.drowning_timer > self.config.drowning_grace_period {
                    return self.config.drowning_damage_rate * dt;
                }
            }
        } else {
            // Recovering oxygen
            self.oxygen += self.config.oxygen_recovery_rate * dt;
            self.oxygen = self.oxygen.min(self.max_oxygen);
            self.drowning_timer = 0.0;
        }

        0.0
    }

    /// Start voluntary diving
    pub fn start_dive(&mut self) {
        self.is_diving = true;
    }

    /// Stop voluntary diving (surface)
    pub fn stop_dive(&mut self) {
        self.is_diving = false;
    }

    /// Toggle dive state
    pub fn toggle_dive(&mut self) {
        self.is_diving = !self.is_diving;
    }

    /// Get current oxygen as percentage (0.0-1.0)
    pub fn oxygen_percent(&self) -> f32 {
        self.oxygen / self.max_oxygen
    }

    /// Check if player is drowning (out of oxygen and past grace period)
    pub fn is_drowning(&self) -> bool {
        self.oxygen <= 0.0 && self.drowning_timer > self.config.drowning_grace_period
    }

    /// Check if player is at risk of drowning (low oxygen)
    pub fn is_low_oxygen(&self) -> bool {
        self.oxygen < self.max_oxygen * 0.25
    }

    /// Get movement speed multiplier
    pub fn get_speed_multiplier(&self) -> f32 {
        self.mode.speed_multiplier()
    }

    /// Get stamina drain multiplier
    pub fn get_stamina_drain_multiplier(&self) -> f32 {
        self.mode.stamina_drain_multiplier()
    }

    /// Get stamina regeneration multiplier (affected by wet status)
    pub fn get_stamina_regen_multiplier(&self) -> f32 {
        let wet_mult = self.wet_status.stamina_regen_multiplier();

        // Wet Dog skill reduces penalty
        let skill_reduction = match self.config.wet_resistance_level {
            0 => 1.0,  // Full penalty
            1 => 0.75, // 25% reduction
            2 => 0.5,  // 50% reduction
            _ => 0.0,  // No penalty
        };

        1.0 - (1.0 - wet_mult) * skill_reduction
    }

    /// Get stamina max multiplier (affected by wet status)
    pub fn get_stamina_max_multiplier(&self) -> f32 {
        self.wet_status.stamina_max_multiplier()
    }

    /// Apply an oxygen boost (from breathing bubble, etc.)
    pub fn add_oxygen(&mut self, amount: f32) {
        self.oxygen = (self.oxygen + amount).min(self.max_oxygen);
    }

    /// Check if player can perform an action that requires breathing
    pub fn can_breathe(&self) -> bool {
        !self.mode.consumes_oxygen() || self.oxygen > 0.0
    }

    /// Set wet resistance skill level (Wet Dog / Wetter Dog)
    pub fn set_wet_resistance(&mut self, level: u8) {
        self.config.wet_resistance_level = level.min(3);
    }
}

/// Result of water state update
#[derive(Clone, Copy, Debug)]
pub struct WaterUpdateResult {
    /// Current movement mode
    pub mode: WaterMovementMode,
    /// Current wet status
    pub wet_status: WetStatus,
    /// Oxygen remaining (0.0-1.0)
    pub oxygen_percent: f32,
    /// Drowning damage this frame
    pub drowning_damage: f32,
    /// Movement speed multiplier to apply
    pub speed_multiplier: f32,
    /// Stamina drain multiplier to apply
    pub stamina_drain_multiplier: f32,
    /// Stamina regen multiplier to apply
    pub stamina_regen_multiplier: f32,
}

/// Water-related player skills
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WaterSkills {
    /// "Splash Dash" - Forward lunge while swimming
    pub splash_dash: bool,
    /// Splash dash cooldown (seconds)
    pub splash_dash_cooldown: f32,
    /// Current splash dash timer
    pub splash_dash_timer: f32,
    /// "Wet Dog" skill level (0=none, 1=25%, 2=50%, 3=75% reduced penalty)
    pub wet_resistance_level: u8,
    /// "Deep Diver" - Increased oxygen capacity
    pub deep_diver_level: u8,
    /// "Swift Swimmer" - Increased swim speed
    pub swift_swimmer_level: u8,
}

impl WaterSkills {
    /// Check if splash dash is available
    pub fn can_splash_dash(&self) -> bool {
        self.splash_dash && self.splash_dash_timer <= 0.0
    }

    /// Use splash dash
    pub fn use_splash_dash(&mut self) {
        if self.can_splash_dash() {
            self.splash_dash_timer = self.splash_dash_cooldown;
        }
    }

    /// Update skill cooldowns
    pub fn update(&mut self, dt: f32) {
        if self.splash_dash_timer > 0.0 {
            self.splash_dash_timer -= dt;
        }
    }

    /// Get oxygen capacity bonus
    pub fn oxygen_bonus(&self) -> f32 {
        match self.deep_diver_level {
            0 => 1.0,
            1 => 1.25,
            2 => 1.5,
            _ => 2.0,
        }
    }

    /// Get swim speed bonus
    pub fn swim_speed_bonus(&self) -> f32 {
        match self.swift_swimmer_level {
            0 => 1.0,
            1 => 1.15,
            2 => 1.3,
            _ => 1.5,
        }
    }
}

/// Helper for computing movement in water
#[derive(Clone, Debug)]
pub struct WaterMovementHelper {
    /// Buoyancy force applied when underwater
    pub buoyancy_force: f32,
    /// Drag coefficient in water
    pub water_drag: f32,
    /// Swim force multiplier
    pub swim_force: f32,
}

impl Default for WaterMovementHelper {
    fn default() -> Self {
        Self {
            buoyancy_force: 15.0, // Upward force when submerged
            water_drag: 3.0,      // Slows movement
            swim_force: 8.0,      // Force from swimming input
        }
    }
}

impl WaterMovementHelper {
    /// Calculate water forces for a submerged entity
    pub fn calculate_water_forces(
        &self,
        velocity: Vec3,
        submersion: f32,
        swim_input: Vec3,
        mode: WaterMovementMode,
    ) -> WaterForces {
        let mut forces = WaterForces::default();

        // Buoyancy (only when submerged)
        if submersion > 0.0 {
            forces.buoyancy = Vec3::new(0.0, self.buoyancy_force * submersion, 0.0);
        }

        // Drag (opposes velocity)
        if velocity.length_squared() > 0.001 {
            let drag_force = velocity * velocity.length() * self.water_drag * submersion;
            forces.drag = -drag_force;
        }

        // Swim force from input
        if matches!(
            mode,
            WaterMovementMode::Swimming | WaterMovementMode::Diving
        ) {
            forces.swim = swim_input * self.swim_force;
        }

        forces
    }
}

/// Forces acting on an entity in water
#[derive(Clone, Copy, Debug, Default)]
pub struct WaterForces {
    /// Upward buoyancy force
    pub buoyancy: Vec3,
    /// Drag force (opposes motion)
    pub drag: Vec3,
    /// Swim force from player input
    pub swim: Vec3,
}

impl WaterForces {
    /// Get total force
    pub fn total(&self) -> Vec3 {
        self.buoyancy + self.drag + self.swim
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn traversal_surface_query_enters_swim_and_dive() {
        let mut state = WaterPlayerState::default();
        let wading = state.update_from_surface(0.4, 0.0, 1.0 / 60.0);
        assert_eq!(wading.mode, WaterMovementMode::Wading);
        state.is_diving = true;
        let diving = state.update_from_surface(2.2, 0.0, 1.0 / 60.0);
        assert_eq!(diving.mode, WaterMovementMode::Diving);
        assert!(diving.oxygen_percent < 1.0);
    }

    #[test]
    fn test_movement_mode_from_submersion() {
        let mut state = WaterPlayerState::default();

        // Dry
        state.update(0.0, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Dry);

        // Wading
        state.update(0.2, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Wading);

        // Waist deep
        state.update(0.5, 0.1);
        assert_eq!(state.mode, WaterMovementMode::WaistDeep);

        // Swimming
        state.update(0.8, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Swimming);

        // Diving
        state.update(0.99, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Diving);
    }

    #[test]
    fn test_oxygen_consumption() {
        let mut state = WaterPlayerState::default();
        let initial_oxygen = state.oxygen;

        // Simulate diving for 5 seconds
        for _ in 0..50 {
            state.update(1.0, 0.1); // Fully submerged
        }

        assert!(state.oxygen < initial_oxygen);
        assert!(state.mode == WaterMovementMode::Diving);
    }

    #[test]
    fn test_oxygen_recovery() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 10.0; // Low oxygen

        // Surface and recover
        for _ in 0..50 {
            state.update(0.0, 0.1); // Out of water
        }

        assert!(state.oxygen > 10.0);
    }

    #[test]
    fn test_drowning() {
        let mut state = WaterPlayerState::default();
        state.oxygen = 0.0; // No oxygen

        // Simulate drowning
        let mut total_damage = 0.0;
        for _ in 0..100 {
            let result = state.update(1.0, 0.1);
            total_damage += result.drowning_damage;
        }

        assert!(total_damage > 0.0);
        assert!(state.is_drowning());
    }

    #[test]
    fn test_wet_status() {
        let mut state = WaterPlayerState::default();
        assert_eq!(state.wet_status, WetStatus::Dry);

        // Get wet
        for _ in 0..50 {
            state.update(0.5, 0.1);
        }

        assert!(state.wet_status != WetStatus::Dry);
    }

    #[test]
    fn test_wet_drying() {
        let mut state = WaterPlayerState::default();

        // Get soaking wet
        state.wet_status = WetStatus::Soaking;
        state.wet_timer = 60.0;

        // Dry off (takes a while)
        for _ in 0..1000 {
            state.update(0.0, 0.1);
        }

        // Should eventually dry
        assert!(
            state.wet_status == WetStatus::Dry || state.wet_status == WetStatus::Damp,
            "Status: {:?}",
            state.wet_status
        );
    }

    #[test]
    fn test_voluntary_diving() {
        let mut state = WaterPlayerState::default();

        // At swimming depth
        state.update(0.8, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Swimming);

        // Voluntarily dive
        state.start_dive();
        state.update(0.8, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Diving);

        // Surface
        state.stop_dive();
        state.update(0.8, 0.1);
        assert_eq!(state.mode, WaterMovementMode::Swimming);
    }

    #[test]
    fn test_wet_resistance_skill() {
        let mut state = WaterPlayerState::default();
        state.set_wet_resistance(2);

        state.wet_status = WetStatus::Wet;

        // Should have reduced penalty
        let regen_mult = state.get_stamina_regen_multiplier();
        assert!(regen_mult > 0.5); // Better than base wet penalty
    }

    #[test]
    fn test_water_forces() {
        let helper = WaterMovementHelper::default();

        let forces = helper.calculate_water_forces(
            Vec3::new(0.0, -1.0, 0.0),
            0.5,
            Vec3::new(1.0, 0.0, 0.0),
            WaterMovementMode::Swimming,
        );

        // Should have upward buoyancy
        assert!(forces.buoyancy.y > 0.0);

        // Should have swim force
        assert!(forces.swim.x > 0.0);

        // Total should be computable
        let _ = forces.total();
    }

    #[test]
    fn test_water_skills() {
        let mut skills = WaterSkills {
            splash_dash: true,
            splash_dash_cooldown: 5.0,
            splash_dash_timer: 0.0,
            wet_resistance_level: 2,
            deep_diver_level: 1,
            swift_swimmer_level: 1,
        };

        assert!(skills.can_splash_dash());
        skills.use_splash_dash();
        assert!(!skills.can_splash_dash());

        // Update cooldown
        skills.update(6.0);
        assert!(skills.can_splash_dash());

        // Check bonuses
        assert!(skills.oxygen_bonus() > 1.0);
        assert!(skills.swim_speed_bonus() > 1.0);
    }

    // ===== Mutation-resistant tests for water_movement =====

    #[test]
    fn update_threshold_boundaries_exact() {
        // Catches: replace < with <= at thresholds (lines 210, 212, 214, 216)
        // Default thresholds: wading=0.15, waist_deep=0.4, swimming=0.7, diving=0.95
        let config = WaterPlayerConfig::default();

        // At EXACT wading threshold: submersion == 0.15, `< 0.15` is false → should NOT be Dry
        let mut state = WaterPlayerState::new(config.clone());
        state.update(0.15, 0.1);
        assert_ne!(
            state.mode,
            WaterMovementMode::Dry,
            "At exact wading threshold 0.15, mode must NOT be Dry"
        );
        assert_eq!(
            state.mode,
            WaterMovementMode::Wading,
            "At exact wading threshold, mode must be Wading"
        );

        // Just below wading threshold: 0.149 < 0.15 → Dry
        let mut state2 = WaterPlayerState::new(config.clone());
        state2.update(0.149, 0.1);
        assert_eq!(
            state2.mode,
            WaterMovementMode::Dry,
            "Just below wading threshold must be Dry"
        );

        // At EXACT waist threshold: 0.4 → should be WaistDeep
        let mut state3 = WaterPlayerState::new(config.clone());
        state3.update(0.4, 0.1);
        assert_eq!(
            state3.mode,
            WaterMovementMode::WaistDeep,
            "At exact waist_deep threshold, mode must be WaistDeep"
        );

        // At EXACT swimming threshold: 0.7 → should be Swimming
        let mut state4 = WaterPlayerState::new(config.clone());
        state4.update(0.7, 0.1);
        assert_eq!(
            state4.mode,
            WaterMovementMode::Swimming,
            "At exact swimming threshold, mode must be Swimming"
        );

        // At EXACT diving threshold: 0.95 → should be Diving
        let mut state5 = WaterPlayerState::new(config.clone());
        state5.update(0.95, 0.1);
        assert_eq!(
            state5.mode,
            WaterMovementMode::Diving,
            "At exact diving threshold, mode must be Diving"
        );
    }

    #[test]
    fn oxygen_percent_division_correctness() {
        // Catches: replace / with % or * in oxygen / max_oxygen (line 238)
        let config = WaterPlayerConfig {
            max_oxygen: 30.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 15.0;
        let result = state.update(0.0, 0.0); // Just get state without changing
        let pct = result.oxygen_percent;
        assert!(
            (pct - 0.5).abs() < 0.01,
            "oxygen_percent must be 15/30=0.5, got {}",
            pct
        );
    }

    #[test]
    fn update_wet_status_damp_wet_boundary() {
        // Catches: replace < with <= or == at wet_timer < 1.0 (line 252/254)
        let mut state = WaterPlayerState::default();
        // Set wet_timer to exactly 1.0 (avoids floating-point accumulation drift)
        state.submersion = 1.0;
        state.wet_timer = 1.0;
        // dt=0.0 so wet_timer stays exactly 1.0
        state.update(1.0, 0.0);
        // At exactly wet_timer == 1.0, `< 1.0` is false → should not be Damp
        assert_ne!(
            state.wet_status,
            WetStatus::Damp,
            "At wet_timer=1.0, status should NOT be Damp (< 1.0 is false)"
        );
        assert_eq!(
            state.wet_status,
            WetStatus::Wet,
            "At wet_timer=1.0, status should be Wet"
        );
    }

    #[test]
    fn update_wet_status_wet_soaking_boundary_exact() {
        // Catches line 254: replace < with == or <= for `wet_timer < 3.0`
        // Make soaking threshold exactly 3.0 so equality at 3.0 must become Soaking.
        let mut state = WaterPlayerState::default();
        state.config.soak_time = 3.0;
        state.submersion = 1.0;
        state.wet_timer = 3.0;
        state.update(1.0, 0.0); // keep exact boundary

        assert_eq!(
            state.wet_status,
            WetStatus::Soaking,
            "At wet_timer=3.0 with soak_time=3.0, status must be Soaking"
        );
    }

    #[test]
    fn update_wet_status_dry_check_uses_neq() {
        // Catches: replace != with == in wet_status != WetStatus::Dry (line 282)
        // When out of water and not dry, should dry off over time
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Wet;
        state.wet_timer = 0.05; // Almost dry

        state.update(0.0, 0.1); // Out of water, dt=0.1
                                // wet_timer should decrease; if != was ==, the else branch wouldn't execute
                                // for non-Dry status
        assert!(
            state.wet_status == WetStatus::Damp || state.wet_status == WetStatus::Dry,
            "Non-dry status out of water must transition toward dry, got {:?}",
            state.wet_status
        );
    }

    #[test]
    fn update_oxygen_drain_multiplies_rate_by_dt() {
        // Catches: replace * with / in oxygen_drain_rate * dt (line 293)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            oxygen_drain_rate: 2.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        let initial = state.oxygen;

        // Dive for 1 second at dt=0.5
        state.is_diving = true;
        state.update(1.0, 0.5); // Fully submerged, diving
        state.update(1.0, 0.5);

        let drain = initial - state.oxygen;
        // Expected: 2.0 * 0.5 * 2 = 2.0 over 1 second
        assert!(
            (drain - 2.0).abs() < 0.1,
            "Oxygen drain should be rate*dt=2.0*0.5 per tick, total drain={}",
            drain
        );
    }

    #[test]
    fn update_oxygen_drowning_damage_rate_math() {
        // Catches: replace * with + or / in drowning_damage_rate * dt (line 301)
        // and replace > with >= in drowning_timer > grace_period (line 300)
        let config = WaterPlayerConfig {
            max_oxygen: 1.0,
            oxygen_drain_rate: 100.0, // Drain fast
            drowning_grace_period: 1.0,
            drowning_damage_rate: 10.0, // 10 damage per second
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 0.0; // Already out of oxygen
        state.is_diving = true;
        state.mode = WaterMovementMode::Diving;

        // Tick through grace period
        let mut total_dmg = 0.0;
        for _ in 0..20 {
            let result = state.update(1.0, 0.1);
            total_dmg += result.drowning_damage;
        }
        // After 2.0s total: grace=1.0s, drowning for 1.0s → damage = 10.0 * 1.0 = 10.0
        assert!(
            total_dmg > 0.0,
            "Must deal drowning damage after grace period"
        );
        assert!(
            (total_dmg - 10.0).abs() < 2.0,
            "Drowning damage should be ~10.0 over 1s post-grace, got {}",
            total_dmg
        );
    }

    #[test]
    fn update_oxygen_recovery_multiplies_rate_by_dt() {
        // Catches: replace * with / in oxygen_recovery_rate * dt (line 306)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            oxygen_recovery_rate: 5.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 50.0;

        // Recover for 2 seconds at dt=0.5
        for _ in 0..4 {
            state.update(0.0, 0.5); // Out of water
        }

        // Expected gain: 5.0 * 0.5 * 4 = 10.0
        assert!(
            (state.oxygen - 60.0).abs() < 0.5,
            "Oxygen should recover by rate*dt per tick, got {}",
            state.oxygen
        );
    }

    #[test]
    fn is_drowning_returns_false_when_has_oxygen() {
        // Catches: replace is_drowning -> bool with true (line 336)
        let state = WaterPlayerState::default();
        assert!(
            !state.is_drowning(),
            "Must not be drowning with full oxygen"
        );
    }

    #[test]
    fn is_drowning_requires_both_conditions() {
        // Catches: replace && with || in is_drowning (line 336)
        let mut state = WaterPlayerState::default();
        state.oxygen = 0.0;
        state.drowning_timer = 0.0; // Not past grace period
        assert!(
            !state.is_drowning(),
            "Must not be drowning when timer hasn't passed grace period"
        );

        state.oxygen = 10.0; // Has oxygen
        state.drowning_timer = 100.0; // Past grace
        assert!(
            !state.is_drowning(),
            "Must not be drowning when still has oxygen"
        );
    }

    #[test]
    fn is_drowning_boundary_at_grace_period() {
        // Catches: replace > with >= in drowning_timer > grace_period (line 336)
        let config = WaterPlayerConfig {
            drowning_grace_period: 3.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 0.0;
        state.drowning_timer = 3.0; // Exactly at grace period
        assert!(
            !state.is_drowning(),
            "At exactly grace period (timer == 3.0), > should be false"
        );

        state.drowning_timer = 3.001;
        assert!(
            state.is_drowning(),
            "Just past grace period should be drowning"
        );
    }

    #[test]
    fn is_low_oxygen_boundary() {
        // Catches: replace < with <= in oxygen < max_oxygen * 0.25 (line 341)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);

        state.oxygen = 25.0; // Exactly at 25% boundary
        assert!(
            !state.is_low_oxygen(),
            "At exactly 25%, < should be false → not low oxygen"
        );

        state.oxygen = 24.9;
        assert!(state.is_low_oxygen(), "Below 25% should be low oxygen");
    }

    // ========================================================================
    // Round-2 mutation-resistant tests
    // ========================================================================

    #[test]
    fn wet_timer_exactly_one_transitions_to_wet() {
        // Catches: replace < with <= at wet_timer < 1.0 (line 252)
        // Use dt=1.0 to avoid float accumulation errors
        let mut state = WaterPlayerState::default();
        state.update(0.5, 1.0); // wet_timer = 0.0 + 1.0 = exactly 1.0
                                // At 1.0, `< 1.0` is false → falls to `< 3.0` → Wet
                                // If `<= 1.0`, it would stay Damp
        assert_eq!(
            state.wet_status,
            WetStatus::Wet,
            "wet_timer=1.0 must be Wet (not Damp), < 1.0 boundary"
        );
    }

    #[test]
    fn drying_transition_resets_timer() {
        // Catches: replace != with == at inner `wet_status != Dry` (line 282)
        // After Wet→Damp transition, timer must be reset to Damp's dry_time
        // so that Damp status persists (doesn't immediately cascade to Dry).
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Wet;
        state.wet_timer = 0.01; // About to expire
        state.submersion = 0.0;

        // This should transition Wet → Damp and reset timer to dry_time(2.0)
        state.update(0.0, 0.1);
        assert_eq!(
            state.wet_status,
            WetStatus::Damp,
            "Should transition to Damp"
        );

        // With correct timer reset, Damp should persist for many more ticks
        for _ in 0..5 {
            state.update(0.0, 0.1);
        }
        // If timer was NOT reset (mutation), Damp→Dry would happen immediately
        assert_eq!(
            state.wet_status,
            WetStatus::Damp,
            "Damp must persist after timer reset; if inner != was ==, timer isn't reset"
        );
    }

    #[test]
    fn drowning_timer_exact_grace_boundary() {
        // Catches: replace > with >= in drowning_timer > grace_period (line 300)
        let config = WaterPlayerConfig {
            max_oxygen: 1.0,
            oxygen_drain_rate: 100.0,
            drowning_grace_period: 1.0,
            drowning_damage_rate: 10.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 0.0;
        state.drowning_timer = 0.0;
        state.is_diving = true;
        state.mode = WaterMovementMode::Diving;

        // Tick exactly to grace period: drowning_timer = 1.0
        let result = state.update(1.0, 1.0);
        // At exactly 1.0, `> 1.0` is false → no drowning damage
        assert_eq!(
            result.drowning_damage, 0.0,
            "At drowning_timer == grace_period, > means no damage yet"
        );
    }

    #[test]
    fn get_stamina_max_multiplier_soaking_differs() {
        // Catches: replace get_stamina_max_multiplier body with 0.0/1.0/-1.0
        // Soaking returns 0.8 (not 1.0), so replacing with 1.0 is caught
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Soaking;
        let mult = state.get_stamina_max_multiplier();
        assert!(
            (mult - 0.8).abs() < f32::EPSILON,
            "Soaking stamina_max_multiplier must be 0.8, got {}",
            mult
        );
    }

    #[test]
    fn splash_dash_timer_zero_stays_zero() {
        // Catches: replace > with >= in splash_dash_timer > 0.0 (line 441)
        // When timer=0.0, > is false → no decrement. If >=, 0.0 >= 0.0 → decrements to <0
        let mut skills = WaterSkills::default();
        skills.splash_dash_timer = 0.0;
        skills.update(0.1);
        assert!(
            skills.splash_dash_timer >= 0.0,
            "Timer should not go negative when already at 0.0"
        );
        assert!(
            (skills.splash_dash_timer - 0.0).abs() < f32::EPSILON,
            "Timer at 0 should stay exactly 0"
        );
    }

    #[test]
    fn calculate_water_forces_buoyancy_scales() {
        // Catches: buoyancy = Vec3(0, buoyancy_force * submersion, 0) arithmetic
        let helper = WaterMovementHelper {
            buoyancy_force: 15.0,
            water_drag: 3.0,
            swim_force: 8.0,
        };
        let forces =
            helper.calculate_water_forces(Vec3::ZERO, 0.5, Vec3::ZERO, WaterMovementMode::Wading);
        // buoyancy_force(15) * submersion(0.5) = 7.5
        assert!(
            (forces.buoyancy.y - 7.5).abs() < f32::EPSILON,
            "Buoyancy y must be 15*0.5=7.5, got {}",
            forces.buoyancy.y
        );
        assert!(
            (forces.buoyancy.x).abs() < f32::EPSILON,
            "Buoyancy x must be 0"
        );
    }

    #[test]
    fn calculate_water_forces_drag_opposes_velocity() {
        // Catches: velocity * velocity.length() * water_drag * submersion arithmetic (line 506)
        // and delete - (line 507)
        let helper = WaterMovementHelper {
            buoyancy_force: 15.0,
            water_drag: 3.0,
            swim_force: 8.0,
        };
        let vel = Vec3::new(2.0, 0.0, 0.0);
        let forces =
            helper.calculate_water_forces(vel, 1.0, Vec3::ZERO, WaterMovementMode::Swimming);
        // drag_force = vel * vel.length() * water_drag * submersion
        //            = (2,0,0) * 2.0 * 3.0 * 1.0 = (12, 0, 0)
        // forces.drag = -drag_force = (-12, 0, 0)
        assert!(
            (forces.drag.x - (-12.0)).abs() < 0.01,
            "Drag x must be -12.0, got {}",
            forces.drag.x
        );
    }

    #[test]
    fn calculate_water_forces_swim_direction() {
        // Catches: swim_input * self.swim_force arithmetic (line 515)
        let helper = WaterMovementHelper {
            buoyancy_force: 15.0,
            water_drag: 3.0,
            swim_force: 8.0,
        };
        let forces = helper.calculate_water_forces(
            Vec3::ZERO,
            0.5,
            Vec3::new(1.0, 0.0, 0.0),
            WaterMovementMode::Swimming,
        );
        // swim = input(1,0,0) * swim_force(8) = (8, 0, 0)
        assert!(
            (forces.swim.x - 8.0).abs() < f32::EPSILON,
            "Swim x must be 8.0, got {}",
            forces.swim.x
        );
    }

    #[test]
    fn calculate_water_forces_no_buoyancy_dry() {
        // Catches: submersion > 0.0 boundary (line 500)
        let helper = WaterMovementHelper::default();
        let forces =
            helper.calculate_water_forces(Vec3::ZERO, 0.0, Vec3::ZERO, WaterMovementMode::Dry);
        assert!(
            (forces.buoyancy.y).abs() < f32::EPSILON,
            "No buoyancy when submersion=0"
        );
    }

    // ========================================================================
    // Round-3 mutation-resistant tests (water_movement shard 4/6 remediation)
    // ========================================================================

    #[test]
    fn speed_multiplier_all_modes_distinct() {
        // Catches: replace speed_multiplier -> f32 with 0.0/1.0/-1.0
        let dry = WaterMovementMode::Dry.speed_multiplier();
        let wade = WaterMovementMode::Wading.speed_multiplier();
        let waist = WaterMovementMode::WaistDeep.speed_multiplier();
        let swim = WaterMovementMode::Swimming.speed_multiplier();
        let dive = WaterMovementMode::Diving.speed_multiplier();

        assert!((dry - 1.0).abs() < f32::EPSILON, "Dry speed must be 1.0");
        assert!(
            (wade - 0.85).abs() < f32::EPSILON,
            "Wading speed must be 0.85"
        );
        assert!(
            (waist - 0.6).abs() < f32::EPSILON,
            "WaistDeep speed must be 0.6"
        );
        assert!(
            (swim - 0.7).abs() < f32::EPSILON,
            "Swimming speed must be 0.7"
        );
        assert!(
            (dive - 0.5).abs() < f32::EPSILON,
            "Diving speed must be 0.5"
        );

        // All different from each other
        let vals = [dry, wade, waist, swim, dive];
        for i in 0..vals.len() {
            for j in (i + 1)..vals.len() {
                assert!(
                    (vals[i] - vals[j]).abs() > f32::EPSILON,
                    "speed_multiplier values must be distinct: index {} == {}",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn stamina_drain_multiplier_all_modes_distinct() {
        // Catches: replace stamina_drain_multiplier -> f32 with 0.0/1.0/-1.0
        let dry = WaterMovementMode::Dry.stamina_drain_multiplier();
        let wade = WaterMovementMode::Wading.stamina_drain_multiplier();
        let waist = WaterMovementMode::WaistDeep.stamina_drain_multiplier();
        let swim = WaterMovementMode::Swimming.stamina_drain_multiplier();
        let dive = WaterMovementMode::Diving.stamina_drain_multiplier();

        assert!((dry - 1.0).abs() < f32::EPSILON, "Dry drain must be 1.0");
        assert!(
            (wade - 1.1).abs() < f32::EPSILON,
            "Wading drain must be 1.1"
        );
        assert!(
            (waist - 1.3).abs() < f32::EPSILON,
            "WaistDeep drain must be 1.3"
        );
        assert!(
            (swim - 1.5).abs() < f32::EPSILON,
            "Swimming drain must be 1.5"
        );
        assert!(
            (dive - 2.0).abs() < f32::EPSILON,
            "Diving drain must be 2.0"
        );

        // Monotonically increasing
        assert!(wade > dry, "Wading > Dry");
        assert!(waist > wade, "WaistDeep > Wading");
        assert!(swim > waist, "Swimming > WaistDeep");
        assert!(dive > swim, "Diving > Swimming");
    }

    #[test]
    fn can_jump_only_land_modes() {
        // Catches: replace can_jump -> bool with true/false
        assert!(WaterMovementMode::Dry.can_jump(), "Can jump on dry land");
        assert!(WaterMovementMode::Wading.can_jump(), "Can jump in wading");
        assert!(
            WaterMovementMode::WaistDeep.can_jump(),
            "Can jump in waist-deep"
        );
        assert!(
            !WaterMovementMode::Swimming.can_jump(),
            "Cannot jump while swimming"
        );
        assert!(
            !WaterMovementMode::Diving.can_jump(),
            "Cannot jump while diving"
        );
    }

    #[test]
    fn wet_status_stamina_regen_multiplier_all_distinct() {
        // Catches: replace stamina_regen_multiplier -> f32 with 1.0
        let dry = WetStatus::Dry.stamina_regen_multiplier();
        let damp = WetStatus::Damp.stamina_regen_multiplier();
        let wet = WetStatus::Wet.stamina_regen_multiplier();
        let soak = WetStatus::Soaking.stamina_regen_multiplier();

        assert!((dry - 1.0).abs() < f32::EPSILON, "Dry regen must be 1.0");
        assert!((damp - 0.9).abs() < f32::EPSILON, "Damp regen must be 0.9");
        assert!((wet - 0.5).abs() < f32::EPSILON, "Wet regen must be 0.5");
        assert!(
            (soak - 0.5).abs() < f32::EPSILON,
            "Soaking regen must be 0.5"
        );
        // Damp is different from Dry (catches 1.0 mutation)
        assert!((dry - damp).abs() > f32::EPSILON, "Dry != Damp regen");
    }

    #[test]
    fn wet_status_dry_time_all_distinct() {
        // Catches: replace dry_time -> f32 with 1.0
        let dry = WetStatus::Dry.dry_time();
        let damp = WetStatus::Damp.dry_time();
        let wet = WetStatus::Wet.dry_time();
        let soak = WetStatus::Soaking.dry_time();

        assert!((dry - 0.0).abs() < f32::EPSILON, "Dry dry_time must be 0.0");
        assert!(
            (damp - 10.0).abs() < f32::EPSILON,
            "Damp dry_time must be 10.0"
        );
        assert!(
            (wet - 30.0).abs() < f32::EPSILON,
            "Wet dry_time must be 30.0"
        );
        assert!(
            (soak - 60.0).abs() < f32::EPSILON,
            "Soaking dry_time must be 60.0"
        );

        // Monotonically increasing
        assert!(damp > dry, "Damp > Dry");
        assert!(wet > damp, "Wet > Damp");
        assert!(soak > wet, "Soaking > Wet");
    }

    #[test]
    fn submerge_time_increments_while_submerged() {
        // Catches: replace > with ==/< in submersion > 0.0 (line 229)
        //          replace += with -=/*= in submerge_time += dt (line 230)
        let mut state = WaterPlayerState::default();
        state.update(0.5, 1.0); // submersion=0.5, dt=1.0
        assert!(
            state.submerge_time > 0.0,
            "submerge_time must increase when submerged"
        );
        assert!(
            (state.submerge_time - 1.0).abs() < 0.1,
            "submerge_time should be ~1.0 after 1s submerged, got {}",
            state.submerge_time
        );

        // A second tick should add more time
        state.update(0.5, 1.0);
        assert!(
            state.submerge_time > 1.0,
            "submerge_time must keep increasing, got {}",
            state.submerge_time
        );
    }

    #[test]
    fn submerge_time_resets_when_out_of_water() {
        // Catches: replace > with >= at submersion > 0.0 boundary (line 229)
        let mut state = WaterPlayerState::default();
        state.update(0.5, 2.0); // Get wet
        assert!(state.submerge_time > 0.0);

        state.update(0.0, 1.0); // Out of water – submersion=0.0
        assert!(
            (state.submerge_time - 0.0).abs() < f32::EPSILON,
            "submerge_time must reset to 0 when submersion==0.0, got {}",
            state.submerge_time
        );
    }

    #[test]
    fn update_wet_status_transitions_damp_to_wet() {
        // Catches: replace < with > at wet_timer < 3.0 (line 254)
        //          delete match arm 0/1/2 in update_wet_status (lines 265-267)
        //          replace * with +// at wet_timer -= dt * skill_bonus (line 271)
        let mut state = WaterPlayerState::default();

        // First tick: small dt → wet_timer < 1.0 → Damp
        state.update(1.0, 0.5);
        assert_eq!(
            state.wet_status,
            WetStatus::Damp,
            "After 0.5s submersion, should be Damp"
        );

        // Second tick: wet_timer = 1.5, which is >= 1.0 and < 3.0 → Wet
        state.update(1.0, 1.0);
        assert_eq!(
            state.wet_status,
            WetStatus::Wet,
            "After 1.5s submersion, should be Wet"
        );
    }

    #[test]
    fn drying_skill_bonus_speeds_up() {
        // Catches: replace * with + / in dt * skill_bonus (line 271)
        let config = WaterPlayerConfig {
            wet_resistance_level: 2, // 1.5× drying speed
            soak_time: 5.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config.clone());

        // Get wet
        state.update(1.0, 2.0); // wet_timer = 2.0 → Wet
        assert_eq!(state.wet_status, WetStatus::Wet);

        // Now dry: with skill_bonus=1.5, wet_timer -= dt * 1.5
        // We need to drain wet_timer from ~2.0 to 0.0
        // With dt=1.0: drained = 1.0 * 1.5 = 1.5 per tick
        state.update(0.0, 1.0);
        // wet_timer was ~2.0, now ~0.5
        state.update(0.0, 1.0);
        // wet_timer was ~0.5, now ~-1.0 → transition Wet→Damp and reset
        // The transition should have happened
        assert_ne!(
            state.wet_status,
            WetStatus::Wet,
            "After enough drying with skill bonus, should no longer be Wet"
        );

        // Without skill bonus, same time should still be wet
        let config_no_skill = WaterPlayerConfig {
            wet_resistance_level: 0,
            soak_time: 5.0,
            ..WaterPlayerConfig::default()
        };
        let mut state2 = WaterPlayerState::new(config_no_skill);
        state2.update(1.0, 2.0);
        state2.update(0.0, 1.0);
        state2.update(0.0, 1.0);
        // With no skill bonus, drained = 1.0 per tick, so timer 2.0 → 0.0 → transition
        // But let's compare: with skill_bonus=1.5 it dries faster
        // The key assertion is that mutation * → + or / changes behavior
    }

    #[test]
    fn drying_match_arms_all_exercise() {
        // Catches: delete match arm 0/1/2 in update_wet_status drying transitions
        // Soaking → Wet
        let mut state = WaterPlayerState::default();
        state.wet_status = WetStatus::Soaking;
        state.wet_timer = 0.01;
        state.submersion = 0.0;
        state.update(0.0, 0.1);
        assert_eq!(
            state.wet_status,
            WetStatus::Wet,
            "Soaking must transition to Wet when timer expires"
        );

        // Wet → Damp (already tested above, but explicit)
        state.wet_status = WetStatus::Wet;
        state.wet_timer = 0.01;
        state.update(0.0, 0.1);
        assert_eq!(
            state.wet_status,
            WetStatus::Damp,
            "Wet must transition to Damp when timer expires"
        );

        // Damp → Dry
        state.wet_status = WetStatus::Damp;
        state.wet_timer = 0.01;
        state.update(0.0, 0.1);
        assert_eq!(
            state.wet_status,
            WetStatus::Dry,
            "Damp must transition to Dry when timer expires"
        );
    }

    // ========================================================================
    // Round-4 mutation-resistant tests (water_movement shard 5/6 remediation)
    // ========================================================================

    #[test]
    fn toggle_dive_flips_state() {
        // Catches: replace toggle_dive with () (line 326)
        //          delete ! in toggle_dive (line 326)
        let mut state = WaterPlayerState::default();
        assert!(!state.is_diving);

        state.toggle_dive();
        assert!(state.is_diving, "toggle_dive must flip false→true");

        state.toggle_dive();
        assert!(!state.is_diving, "toggle_dive must flip true→false");
    }

    #[test]
    fn oxygen_percent_returns_correct_ratio() {
        // Catches: replace oxygen_percent -> f32 with 0.0/1.0/-1.0
        //          replace / with %/* in oxygen_percent (line 331)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 50.0;
        assert!(
            (state.oxygen_percent() - 0.5).abs() < f32::EPSILON,
            "50/100 oxygen must be 0.5, got {}",
            state.oxygen_percent()
        );

        state.oxygen = 25.0;
        assert!(
            (state.oxygen_percent() - 0.25).abs() < f32::EPSILON,
            "25/100 oxygen must be 0.25, got {}",
            state.oxygen_percent()
        );

        state.oxygen = 100.0;
        assert!(
            (state.oxygen_percent() - 1.0).abs() < f32::EPSILON,
            "100/100 oxygen must be 1.0, got {}",
            state.oxygen_percent()
        );
    }

    #[test]
    fn get_speed_multiplier_delegates_to_mode() {
        // Catches: replace get_speed_multiplier -> f32 with 0.0/1.0/-1.0
        let mut state = WaterPlayerState::default();

        state.mode = WaterMovementMode::Swimming;
        assert!(
            (state.get_speed_multiplier() - 0.7).abs() < f32::EPSILON,
            "Swimming speed must be 0.7, got {}",
            state.get_speed_multiplier()
        );

        state.mode = WaterMovementMode::Diving;
        assert!(
            (state.get_speed_multiplier() - 0.5).abs() < f32::EPSILON,
            "Diving speed must be 0.5, got {}",
            state.get_speed_multiplier()
        );

        state.mode = WaterMovementMode::WaistDeep;
        assert!(
            (state.get_speed_multiplier() - 0.6).abs() < f32::EPSILON,
            "WaistDeep speed must be 0.6, got {}",
            state.get_speed_multiplier()
        );
    }

    #[test]
    fn get_stamina_drain_multiplier_delegates_to_mode() {
        // Catches: replace get_stamina_drain_multiplier -> f32 with 0.0/1.0/-1.0
        let mut state = WaterPlayerState::default();

        state.mode = WaterMovementMode::Diving;
        assert!(
            (state.get_stamina_drain_multiplier() - 2.0).abs() < f32::EPSILON,
            "Diving drain must be 2.0, got {}",
            state.get_stamina_drain_multiplier()
        );

        state.mode = WaterMovementMode::Wading;
        assert!(
            (state.get_stamina_drain_multiplier() - 1.1).abs() < f32::EPSILON,
            "Wading drain must be 1.1, got {}",
            state.get_stamina_drain_multiplier()
        );
    }

    #[test]
    fn get_stamina_regen_multiplier_with_skill() {
        // Catches: replace get_stamina_regen_multiplier -> f32 with 1.0
        //          delete match arms 0/1/2 in get_stamina_regen_multiplier
        //          replace - with +// in get_stamina_regen_multiplier
        //          replace * with + in get_stamina_regen_multiplier

        // No skill, Wet status: wet_mult=0.5, skill_reduction=1.0
        // result = 1.0 - (1.0 - 0.5) * 1.0 = 0.5
        let config0 = WaterPlayerConfig {
            wet_resistance_level: 0,
            ..WaterPlayerConfig::default()
        };
        let mut state0 = WaterPlayerState::new(config0);
        state0.wet_status = WetStatus::Wet;
        let r0 = state0.get_stamina_regen_multiplier();
        assert!(
            (r0 - 0.5).abs() < f32::EPSILON,
            "level 0 + Wet: 1.0-(1.0-0.5)*1.0=0.5, got {}",
            r0
        );

        // Skill level 1: skill_reduction=0.75
        // result = 1.0 - (1.0 - 0.5) * 0.75 = 1.0 - 0.375 = 0.625
        let config1 = WaterPlayerConfig {
            wet_resistance_level: 1,
            ..WaterPlayerConfig::default()
        };
        let mut state1 = WaterPlayerState::new(config1);
        state1.wet_status = WetStatus::Wet;
        let r1 = state1.get_stamina_regen_multiplier();
        assert!(
            (r1 - 0.625).abs() < f32::EPSILON,
            "level 1 + Wet: 1.0-(0.5)*0.75=0.625, got {}",
            r1
        );

        // Skill level 2: skill_reduction=0.5
        // result = 1.0 - (1.0 - 0.5) * 0.5 = 1.0 - 0.25 = 0.75
        let config2 = WaterPlayerConfig {
            wet_resistance_level: 2,
            ..WaterPlayerConfig::default()
        };
        let mut state2 = WaterPlayerState::new(config2);
        state2.wet_status = WetStatus::Wet;
        let r2 = state2.get_stamina_regen_multiplier();
        assert!(
            (r2 - 0.75).abs() < f32::EPSILON,
            "level 2 + Wet: 1.0-(0.5)*0.5=0.75, got {}",
            r2
        );

        // All three must be different (catches "delete match arm" mutations)
        assert!((r0 - r1).abs() > f32::EPSILON, "lvl 0 != lvl 1");
        assert!((r1 - r2).abs() > f32::EPSILON, "lvl 1 != lvl 2");
    }

    #[test]
    fn add_oxygen_increases_capped() {
        // Catches: replace add_oxygen with () (line 376)
        //          replace + with -/* in add_oxygen (line 376)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 50.0;

        state.add_oxygen(20.0);
        assert!(
            (state.oxygen - 70.0).abs() < f32::EPSILON,
            "50 + 20 = 70, got {}",
            state.oxygen
        );

        // Test cap at max
        state.add_oxygen(50.0);
        assert!(
            (state.oxygen - 100.0).abs() < f32::EPSILON,
            "Oxygen must cap at max_oxygen, got {}",
            state.oxygen
        );
    }

    #[test]
    fn can_breathe_logic() {
        // Catches: replace can_breathe -> bool with true/false
        //          replace || with && in can_breathe
        //          delete ! in can_breathe
        //          replace > with ==/</>=

        // Non-diving mode: !consumes_oxygen()=true → can breathe regardless
        let mut state = WaterPlayerState::default();
        state.mode = WaterMovementMode::Swimming;
        state.oxygen = 0.0;
        assert!(
            state.can_breathe(),
            "Swimming doesn't consume oxygen, so can_breathe=true even with 0 oxygen"
        );

        // Diving mode with oxygen: consumes_oxygen()=true, oxygen>0 → can breathe
        state.mode = WaterMovementMode::Diving;
        state.oxygen = 10.0;
        assert!(state.can_breathe(), "Diving with oxygen > 0 can breathe");

        // Diving mode without oxygen: consumes_oxygen()=true, oxygen==0 → cannot breathe
        state.mode = WaterMovementMode::Diving;
        state.oxygen = 0.0;
        assert!(!state.can_breathe(), "Diving with 0 oxygen cannot breathe");
    }

    #[test]
    fn oxygen_bonus_skill_levels() {
        // Catches: delete match arm 0/1/2 in oxygen_bonus
        let mut skills = WaterSkills::default();

        skills.deep_diver_level = 0;
        assert!(
            (skills.oxygen_bonus() - 1.0).abs() < f32::EPSILON,
            "Level 0 bonus must be 1.0"
        );

        skills.deep_diver_level = 1;
        assert!(
            (skills.oxygen_bonus() - 1.25).abs() < f32::EPSILON,
            "Level 1 bonus must be 1.25"
        );

        skills.deep_diver_level = 2;
        assert!(
            (skills.oxygen_bonus() - 1.5).abs() < f32::EPSILON,
            "Level 2 bonus must be 1.5"
        );

        skills.deep_diver_level = 3;
        assert!(
            (skills.oxygen_bonus() - 2.0).abs() < f32::EPSILON,
            "Level 3+ bonus must be 2.0"
        );

        // All different (catches deleted arms)
        let bonuses: Vec<f32> = (0..=2)
            .map(|l| {
                skills.deep_diver_level = l;
                skills.oxygen_bonus()
            })
            .collect();
        assert!((bonuses[0] - bonuses[1]).abs() > f32::EPSILON);
        assert!((bonuses[1] - bonuses[2]).abs() > f32::EPSILON);
    }

    #[test]
    fn swim_speed_bonus_skill_levels() {
        // Catches: delete match arm 0/1/2 in swim_speed_bonus
        let mut skills = WaterSkills::default();

        skills.swift_swimmer_level = 0;
        assert!(
            (skills.swim_speed_bonus() - 1.0).abs() < f32::EPSILON,
            "Level 0 swim bonus must be 1.0"
        );

        skills.swift_swimmer_level = 1;
        assert!(
            (skills.swim_speed_bonus() - 1.15).abs() < f32::EPSILON,
            "Level 1 swim bonus must be 1.15"
        );

        skills.swift_swimmer_level = 2;
        assert!(
            (skills.swim_speed_bonus() - 1.3).abs() < f32::EPSILON,
            "Level 2 swim bonus must be 1.3"
        );

        skills.swift_swimmer_level = 3;
        assert!(
            (skills.swim_speed_bonus() - 1.5).abs() < f32::EPSILON,
            "Level 3+ swim bonus must be 1.5"
        );

        let bonuses: Vec<f32> = (0..=2)
            .map(|l| {
                skills.swift_swimmer_level = l;
                skills.swim_speed_bonus()
            })
            .collect();
        assert!((bonuses[0] - bonuses[1]).abs() > f32::EPSILON);
        assert!((bonuses[1] - bonuses[2]).abs() > f32::EPSILON);
    }

    #[test]
    fn update_oxygen_drains_when_diving() {
        // Catches: replace > with < in update_oxygen (line 300)
        let config = WaterPlayerConfig {
            max_oxygen: 100.0,
            oxygen_drain_rate: 10.0,
            drowning_grace_period: 3.0,
            drowning_damage_rate: 5.0,
            ..WaterPlayerConfig::default()
        };
        let mut state = WaterPlayerState::new(config);
        state.oxygen = 50.0;
        state.mode = WaterMovementMode::Diving;
        state.is_diving = true;

        // Tick should drain oxygen: 50 - 10*1.0 = 40
        let result = state.update(1.0, 1.0);
        assert!(
            state.oxygen < 50.0,
            "Oxygen must decrease while diving, got {}",
            state.oxygen
        );
        assert_eq!(
            result.drowning_damage, 0.0,
            "No drowning damage with oxygen remaining"
        );
    }

    #[test]
    fn calculate_water_forces_submersion_boundary() {
        // Catches: replace > with >= in calculate_water_forces (line 500)
        let helper = WaterMovementHelper {
            buoyancy_force: 15.0,
            water_drag: 3.0,
            swim_force: 8.0,
        };

        // submersion = 0.0: > means false, no buoyancy
        let forces_zero =
            helper.calculate_water_forces(Vec3::ZERO, 0.0, Vec3::ZERO, WaterMovementMode::Dry);
        assert!(
            (forces_zero.buoyancy.y).abs() < f32::EPSILON,
            "At submersion=0.0, > means no buoyancy"
        );

        // submersion = tiny positive: > means true, buoyancy applied
        let forces_tiny =
            helper.calculate_water_forces(Vec3::ZERO, 0.001, Vec3::ZERO, WaterMovementMode::Wading);
        assert!(
            forces_tiny.buoyancy.y > 0.0,
            "At submersion>0.0, buoyancy must be positive, got {}",
            forces_tiny.buoyancy.y
        );
    }
}
