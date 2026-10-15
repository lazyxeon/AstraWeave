// abilities.rs - Player ability system with cooldown management
// Features: Echo Dash (dash attack), Echo Shield (damage reduction), resource costs

use glam::Vec3;

/// Ability types available to the player
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbilityType {
    EchoDash,
    EchoShield,
}

/// Ability state tracking (cooldowns, durations, active status)
#[derive(Debug, Clone)]
pub struct AbilityState {
    pub ability_type: AbilityType,
    pub cooldown_seconds: f32,
    pub duration_seconds: f32, // 0.0 for instant abilities
    pub time_since_use: f32,   // Track cooldown elapsed
    pub time_active: f32,      // Track active duration
    pub is_active: bool,       // Currently in use
    pub echo_cost: u32,        // Resource cost
}

impl AbilityState {
    /// Create new ability state
    pub fn new(ability_type: AbilityType, cooldown: f32, duration: f32, cost: u32) -> Self {
        Self {
            ability_type,
            cooldown_seconds: cooldown,
            duration_seconds: duration,
            time_since_use: cooldown, // Start ready (cooldown elapsed)
            time_active: 0.0,
            is_active: false,
            echo_cost: cost,
        }
    }

    /// Check if ability is off cooldown
    pub fn is_ready(&self) -> bool {
        self.time_since_use >= self.cooldown_seconds
    }

    /// Check if player has enough Echo currency
    pub fn can_afford(&self, player_echo: u32) -> bool {
        player_echo >= self.echo_cost
    }

    /// Activate ability (reset cooldown, set active)
    pub fn activate(&mut self) {
        self.time_since_use = 0.0;
        self.time_active = 0.0;
        self.is_active = self.duration_seconds > 0.0;
    }

    /// Update cooldown and active duration
    pub fn update(&mut self, delta_time: f32) {
        // Update cooldown timer
        if self.time_since_use < self.cooldown_seconds {
            self.time_since_use += delta_time;
        }

        // Update active duration
        if self.is_active {
            self.time_active += delta_time;
            if self.time_active >= self.duration_seconds {
                self.is_active = false;
                self.time_active = 0.0;
            }
        }
    }

    /// Get remaining cooldown time
    pub fn remaining_cooldown(&self) -> f32 {
        (self.cooldown_seconds - self.time_since_use).max(0.0)
    }

    /// Get remaining active time
    pub fn remaining_active(&self) -> f32 {
        if self.is_active {
            (self.duration_seconds - self.time_active).max(0.0)
        } else {
            0.0
        }
    }
}

/// Echo Dash ability - Dash forward and deal damage to first enemy in path
#[derive(Debug, Clone)]
pub struct EchoDash {
    pub state: AbilityState,
    pub damage: f32,
    pub dash_distance: f32,
}

impl EchoDash {
    /// Create new Echo Dash ability
    pub fn new() -> Self {
        Self {
            state: AbilityState::new(AbilityType::EchoDash, 1.0, 0.0, 10), // 1s cooldown, instant, 10 Echo
            damage: 30.0,
            dash_distance: 10.0,
        }
    }

    /// Activate dash (returns dash direction and damage dealt)
    pub fn activate(&mut self, player_pos: Vec3, player_forward: Vec3) -> (Vec3, f32) {
        self.state.activate();
        let target_pos = player_pos + player_forward * self.dash_distance;
        (target_pos, self.damage)
    }

    /// Update cooldown
    pub fn update(&mut self, delta_time: f32) {
        self.state.update(delta_time);
    }

    /// Check if dash is ready
    pub fn can_use(&self, player_echo: u32) -> bool {
        self.state.is_ready() && self.state.can_afford(player_echo)
    }
}

impl Default for EchoDash {
    fn default() -> Self {
        Self::new()
    }
}

/// Echo Shield ability - Reduce incoming damage for duration
#[derive(Debug, Clone)]
pub struct EchoShield {
    pub state: AbilityState,
    pub damage_reduction: f32, // 0.5 = 50% reduction
}

impl EchoShield {
    /// Create new Echo Shield ability
    pub fn new() -> Self {
        Self {
            state: AbilityState::new(AbilityType::EchoShield, 5.0, 3.0, 15), // 5s cooldown, 3s duration, 15 Echo
            damage_reduction: 0.5,
        }
    }

    /// Activate shield
    pub fn activate(&mut self) {
        self.state.activate();
    }

    /// Update shield duration and cooldown
    pub fn update(&mut self, delta_time: f32) {
        self.state.update(delta_time);
    }

    /// Check if shield is ready
    pub fn can_use(&self, player_echo: u32) -> bool {
        self.state.is_ready() && self.state.can_afford(player_echo)
    }

    /// Check if shield is currently active
    pub fn is_active(&self) -> bool {
        self.state.is_active
    }

    /// Apply damage reduction (call when player takes damage)
    pub fn apply_damage_reduction(&self, damage: f32) -> f32 {
        if self.is_active() {
            damage * (1.0 - self.damage_reduction)
        } else {
            damage
        }
    }
}

impl Default for EchoShield {
    fn default() -> Self {
        Self::new()
    }
}

/// Ability manager - Orchestrates all player abilities
#[derive(Debug, Clone)]
pub struct AbilityManager {
    pub echo_dash: EchoDash,
    pub echo_shield: EchoShield,
}

impl AbilityManager {
    /// Create new ability manager with default abilities
    pub fn new() -> Self {
        Self {
            echo_dash: EchoDash::new(),
            echo_shield: EchoShield::new(),
        }
    }

    /// Update all abilities (cooldowns, durations)
    pub fn update(&mut self, delta_time: f32) {
        self.echo_dash.update(delta_time);
        self.echo_shield.update(delta_time);
    }

    pub fn dash_echo_cost(&self) -> u32 {
        self.echo_dash.state.echo_cost
    }

    pub fn shield_echo_cost(&self) -> u32 {
        self.echo_shield.state.echo_cost
    }

    /// Activate Echo Dash (returns dash target position and damage)
    pub fn activate_dash(
        &mut self,
        player_pos: Vec3,
        player_forward: Vec3,
        player_echo: u32,
    ) -> Result<(Vec3, f32), String> {
        if !self.echo_dash.can_use(player_echo) {
            if !self.echo_dash.state.is_ready() {
                return Err(format!(
                    "Echo Dash on cooldown ({:.1}s remaining)",
                    self.echo_dash.state.remaining_cooldown()
                ));
            } else {
                return Err(format!(
                    "Not enough Echo (need {}, have {})",
                    self.echo_dash.state.echo_cost, player_echo
                ));
            }
        }

        Ok(self.echo_dash.activate(player_pos, player_forward))
    }

    /// Activate Echo Shield
    pub fn activate_shield(&mut self, player_echo: u32) -> Result<(), String> {
        if !self.echo_shield.can_use(player_echo) {
            if !self.echo_shield.state.is_ready() {
                return Err(format!(
                    "Echo Shield on cooldown ({:.1}s remaining)",
                    self.echo_shield.state.remaining_cooldown()
                ));
            } else {
                return Err(format!(
                    "Not enough Echo (need {}, have {})",
                    self.echo_shield.state.echo_cost, player_echo
                ));
            }
        }

        self.echo_shield.activate();
        Ok(())
    }

    /// Check if shield is active (for damage reduction)
    pub fn is_shield_active(&self) -> bool {
        self.echo_shield.is_active()
    }

    /// Apply shield damage reduction (call when player takes damage)
    pub fn apply_shield_reduction(&self, damage: f32) -> f32 {
        self.echo_shield.apply_damage_reduction(damage)
    }

    /// Get dash cooldown info
    pub fn dash_cooldown(&self) -> (bool, f32) {
        (
            self.echo_dash.state.is_ready(),
            self.echo_dash.state.remaining_cooldown(),
        )
    }

    /// Get shield cooldown info
    pub fn shield_cooldown(&self) -> (bool, f32) {
        (
            self.echo_shield.state.is_ready(),
            self.echo_shield.state.remaining_cooldown(),
        )
    }

    /// Get shield active info
    pub fn shield_active(&self) -> (bool, f32) {
        (
            self.echo_shield.is_active(),
            self.echo_shield.state.remaining_active(),
        )
    }
}

impl Default for AbilityManager {
    fn default() -> Self {
        Self::new()
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_state_creation() {
        let state = AbilityState::new(AbilityType::EchoDash, 1.0, 0.0, 10);
        assert_eq!(state.ability_type, AbilityType::EchoDash);
        assert_eq!(state.cooldown_seconds, 1.0);
        assert_eq!(state.duration_seconds, 0.0);
        assert_eq!(state.echo_cost, 10);
        assert!(state.is_ready()); // Starts ready
        assert!(!state.is_active);
    }

    #[test]
    fn test_ability_state_cooldown() {
        let mut state = AbilityState::new(AbilityType::EchoDash, 1.0, 0.0, 10);

        // Use ability
        state.activate();
        assert!(!state.is_ready()); // On cooldown
        assert_eq!(state.time_since_use, 0.0);

        // Wait half cooldown
        state.update(0.5);
        assert!(!state.is_ready());
        assert_eq!(state.remaining_cooldown(), 0.5);

        // Wait full cooldown
        state.update(0.5);
        assert!(state.is_ready());
        assert_eq!(state.remaining_cooldown(), 0.0);
    }

    #[test]
    fn test_ability_state_duration() {
        let mut state = AbilityState::new(AbilityType::EchoShield, 5.0, 3.0, 15);

        // Activate (duration-based ability)
        state.activate();
        assert!(state.is_active);
        assert_eq!(state.time_active, 0.0);

        // Wait half duration
        state.update(1.5);
        assert!(state.is_active);
        assert_eq!(state.remaining_active(), 1.5);

        // Wait full duration
        state.update(1.5);
        assert!(!state.is_active); // Expired
        assert_eq!(state.remaining_active(), 0.0);
    }

    #[test]
    fn test_ability_state_affordability() {
        let state = AbilityState::new(AbilityType::EchoDash, 1.0, 0.0, 10);

        assert!(!state.can_afford(5)); // Not enough Echo
        assert!(!state.can_afford(9));
        assert!(state.can_afford(10)); // Exactly enough
        assert!(state.can_afford(20)); // More than enough
    }

    #[test]
    fn test_echo_dash_creation() {
        let dash = EchoDash::new();
        assert_eq!(dash.damage, 30.0);
        assert_eq!(dash.dash_distance, 10.0);
        assert_eq!(dash.state.cooldown_seconds, 1.0);
        assert_eq!(dash.state.echo_cost, 10);
        assert!(dash.state.is_ready());
    }

    #[test]
    fn test_echo_dash_activation() {
        let mut dash = EchoDash::new();
        let player_pos = Vec3::ZERO;
        let player_forward = Vec3::new(0.0, 0.0, 1.0);

        let (target_pos, damage) = dash.activate(player_pos, player_forward);
        assert_eq!(target_pos, Vec3::new(0.0, 0.0, 10.0)); // Dashed forward
        assert_eq!(damage, 30.0);
        assert!(!dash.state.is_ready()); // On cooldown
    }

    #[test]
    fn test_echo_dash_cooldown() {
        let mut dash = EchoDash::new();

        // Use dash
        let _ = dash.activate(Vec3::ZERO, Vec3::Z);
        assert!(!dash.can_use(100)); // Cooldown not ready

        // Wait for cooldown
        dash.update(1.0);
        assert!(dash.can_use(100)); // Ready again
    }

    #[test]
    fn test_echo_dash_cost() {
        let dash = EchoDash::new();

        assert!(!dash.can_use(5)); // Not enough Echo
        assert!(!dash.can_use(9));
        assert!(dash.can_use(10)); // Exactly enough
        assert!(dash.can_use(20)); // More than enough
    }

    #[test]
    fn test_echo_shield_creation() {
        let shield = EchoShield::new();
        assert_eq!(shield.damage_reduction, 0.5);
        assert_eq!(shield.state.cooldown_seconds, 5.0);
        assert_eq!(shield.state.duration_seconds, 3.0);
        assert_eq!(shield.state.echo_cost, 15);
        assert!(shield.state.is_ready());
        assert!(!shield.is_active());
    }

    #[test]
    fn test_echo_shield_activation() {
        let mut shield = EchoShield::new();

        assert!(!shield.is_active());
        shield.activate();
        assert!(shield.is_active());
        assert!(!shield.state.is_ready()); // Cooldown started
    }

    #[test]
    fn test_echo_shield_duration() {
        let mut shield = EchoShield::new();
        shield.activate();

        // Active for 3 seconds
        shield.update(1.5);
        assert!(shield.is_active());

        shield.update(1.5);
        assert!(!shield.is_active()); // Expired
    }

    #[test]
    fn test_echo_shield_damage_reduction() {
        let mut shield = EchoShield::new();

        // No reduction when inactive
        assert_eq!(shield.apply_damage_reduction(100.0), 100.0);

        // 50% reduction when active
        shield.activate();
        assert_eq!(shield.apply_damage_reduction(100.0), 50.0);
        assert_eq!(shield.apply_damage_reduction(50.0), 25.0);

        // No reduction after expiration
        shield.update(3.0);
        assert_eq!(shield.apply_damage_reduction(100.0), 100.0);
    }

    #[test]
    fn test_echo_shield_cost() {
        let shield = EchoShield::new();

        assert!(!shield.can_use(10)); // Not enough Echo
        assert!(!shield.can_use(14));
        assert!(shield.can_use(15)); // Exactly enough
        assert!(shield.can_use(30)); // More than enough
    }

    #[test]
    fn test_ability_manager_creation() {
        let manager = AbilityManager::new();
        assert!(manager.echo_dash.state.is_ready());
        assert!(manager.echo_shield.state.is_ready());
        assert!(!manager.is_shield_active());
    }

    #[test]
    fn test_ability_manager_dash_success() {
        let mut manager = AbilityManager::new();
        let player_pos = Vec3::ZERO;
        let player_forward = Vec3::new(1.0, 0.0, 0.0);

        let result = manager.activate_dash(player_pos, player_forward, 20);
        assert!(result.is_ok());

        let (target_pos, damage) = result.unwrap();
        assert_eq!(target_pos, Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(damage, 30.0);
    }

    #[test]
    fn test_ability_manager_dash_insufficient_echo() {
        let mut manager = AbilityManager::new();
        let result = manager.activate_dash(Vec3::ZERO, Vec3::X, 5);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Not enough Echo"));
        assert!(err.contains("need 10"));
        assert!(err.contains("have 5"));
    }

    #[test]
    fn test_ability_manager_dash_cooldown() {
        let mut manager = AbilityManager::new();

        // First use succeeds
        let result1 = manager.activate_dash(Vec3::ZERO, Vec3::X, 20);
        assert!(result1.is_ok());

        // Second use fails (cooldown)
        let result2 = manager.activate_dash(Vec3::ZERO, Vec3::X, 20);
        assert!(result2.is_err());
        let err = result2.unwrap_err();
        assert!(err.contains("on cooldown"));

        // Wait for cooldown
        manager.update(1.0);
        let result3 = manager.activate_dash(Vec3::ZERO, Vec3::X, 20);
        assert!(result3.is_ok());
    }

    #[test]
    fn test_ability_manager_shield_success() {
        let mut manager = AbilityManager::new();

        let result = manager.activate_shield(20);
        assert!(result.is_ok());
        assert!(manager.is_shield_active());
    }

    #[test]
    fn test_ability_manager_shield_insufficient_echo() {
        let mut manager = AbilityManager::new();
        let result = manager.activate_shield(10);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Not enough Echo"));
        assert!(err.contains("need 15"));
        assert!(err.contains("have 10"));
    }

    #[test]
    fn test_ability_manager_shield_cooldown() {
        let mut manager = AbilityManager::new();

        // First use succeeds
        let result1 = manager.activate_shield(20);
        assert!(result1.is_ok());

        // Second use fails (cooldown)
        let result2 = manager.activate_shield(20);
        assert!(result2.is_err());
        let err = result2.unwrap_err();
        assert!(err.contains("on cooldown"));

        // Wait for cooldown (5 seconds)
        manager.update(5.0);
        let result3 = manager.activate_shield(20);
        assert!(result3.is_ok());
    }

    #[test]
    fn test_ability_manager_shield_duration() {
        let mut manager = AbilityManager::new();
        manager.activate_shield(20).unwrap();

        assert!(manager.is_shield_active());

        // Wait for duration (3 seconds)
        manager.update(3.0);
        assert!(!manager.is_shield_active());
    }

    #[test]
    fn test_ability_manager_damage_reduction() {
        let mut manager = AbilityManager::new();

        // No reduction without shield
        assert_eq!(manager.apply_shield_reduction(100.0), 100.0);

        // Activate shield
        manager.activate_shield(20).unwrap();
        assert_eq!(manager.apply_shield_reduction(100.0), 50.0);

        // Wait for expiration
        manager.update(3.0);
        assert_eq!(manager.apply_shield_reduction(100.0), 100.0);
    }

    #[test]
    fn test_ability_manager_simultaneous_abilities() {
        let mut manager = AbilityManager::new();

        // Use dash
        let dash_result = manager.activate_dash(Vec3::ZERO, Vec3::X, 30);
        assert!(dash_result.is_ok());

        // Use shield (independent cooldown)
        let shield_result = manager.activate_shield(30);
        assert!(shield_result.is_ok());

        // Both on cooldown
        assert!(manager.activate_dash(Vec3::ZERO, Vec3::X, 30).is_err());
        assert!(manager.activate_shield(30).is_err());
    }

    #[test]
    fn test_ability_manager_cooldown_info() {
        let mut manager = AbilityManager::new();

        // Initial state
        let (dash_ready, dash_remaining) = manager.dash_cooldown();
        assert!(dash_ready);
        assert_eq!(dash_remaining, 0.0);

        // After use
        manager.activate_dash(Vec3::ZERO, Vec3::X, 20).unwrap();
        let (dash_ready2, dash_remaining2) = manager.dash_cooldown();
        assert!(!dash_ready2);
        assert_eq!(dash_remaining2, 1.0);

        // After partial wait
        manager.update(0.5);
        let (dash_ready3, dash_remaining3) = manager.dash_cooldown();
        assert!(!dash_ready3);
        assert_eq!(dash_remaining3, 0.5);
    }

    #[test]
    fn test_ability_manager_shield_active_info() {
        let mut manager = AbilityManager::new();

        // Initial state
        let (active, remaining) = manager.shield_active();
        assert!(!active);
        assert_eq!(remaining, 0.0);

        // After activation
        manager.activate_shield(20).unwrap();
        let (active2, remaining2) = manager.shield_active();
        assert!(active2);
        assert_eq!(remaining2, 3.0);

        // After partial duration
        manager.update(1.5);
        let (active3, remaining3) = manager.shield_active();
        assert!(active3);
        assert_eq!(remaining3, 1.5);

        // After expiration
        manager.update(1.5);
        let (active4, remaining4) = manager.shield_active();
        assert!(!active4);
        assert_eq!(remaining4, 0.0);
    }
}
