// Comprehensive action library mapping ActionStep variants to GOAP actions
// Phase 2: Engine Integration

use super::{Action, ActionHistory, StateValue, WorldState};
use std::collections::BTreeMap;

/// Movement action: Move toward a target position
pub struct MoveToAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl MoveToAction {
    pub fn new() -> Self {
        let preconditions = BTreeMap::new();

        let mut effects = BTreeMap::new();
        effects.insert("moved".to_string(), StateValue::Bool(true));
        effects.insert("position_changed".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for MoveToAction {
    fn name(&self) -> &str {
        "move_to"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        2.0
    }
}

/// Approach action: Move closer to enemy while maintaining distance
pub struct ApproachEnemyAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl ApproachEnemyAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("enemy_present".to_string(), StateValue::Bool(true));
        preconditions.insert("enemy_distance".to_string(), StateValue::IntRange(5, 999));

        let mut effects = BTreeMap::new();
        effects.insert("enemy_distance".to_string(), StateValue::IntRange(3, 8));
        effects.insert("in_range".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for ApproachEnemyAction {
    fn name(&self) -> &str {
        "approach_enemy"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        2.5
    }

    fn success_probability(&self, world: &WorldState, history: &ActionHistory) -> f32 {
        let base = history
            .get_action_stats(self.name())
            .map(|s| s.success_rate())
            .unwrap_or(0.85);

        // Higher risk when health is low
        if let Some(StateValue::Int(health)) = world.get("my_health") {
            if *health < 30 {
                return base * 0.7;
            }
        }
        base
    }
}

/// Attack action: Basic attack requiring ammo and close range
pub struct AttackAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl AttackAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("has_ammo".to_string(), StateValue::Bool(true));
        preconditions.insert("enemy_present".to_string(), StateValue::Bool(true));
        preconditions.insert("in_range".to_string(), StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert("enemy_damaged".to_string(), StateValue::Bool(true));
        effects.insert("ammo_spent".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for AttackAction {
    fn name(&self) -> &str {
        "attack"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        5.0
    }

    fn success_probability(&self, world: &WorldState, history: &ActionHistory) -> f32 {
        let base = history
            .get_action_stats(self.name())
            .map(|s| s.success_rate())
            .unwrap_or(0.75);

        // Success depends on health and ammo
        let mut modifier = 1.0;

        if let Some(StateValue::Int(health)) = world.get("my_health") {
            if *health < 40 {
                modifier *= 0.6; // Risky when low health
            }
        }

        if let Some(StateValue::Int(ammo)) = world.get("my_ammo") {
            if *ammo < 5 {
                modifier *= 0.8; // Less reliable when low ammo
            }
        }

        (base * modifier).clamp(0.2, 0.95)
    }
}

/// Cover Fire action: Sustained suppression fire
pub struct CoverFireAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl CoverFireAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("has_ammo".to_string(), StateValue::Bool(true));
        preconditions.insert("enemy_present".to_string(), StateValue::Bool(true));
        preconditions.insert("in_range".to_string(), StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert("enemy_suppressed".to_string(), StateValue::Bool(true));
        effects.insert("cover_provided".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for CoverFireAction {
    fn name(&self) -> &str {
        "cover_fire"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        6.0 // Higher cost due to sustained fire
    }
}

/// Reload action: Restore ammo
pub struct ReloadAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl ReloadAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("has_ammo".to_string(), StateValue::Bool(false));

        let mut effects = BTreeMap::new();
        effects.insert("has_ammo".to_string(), StateValue::Bool(true));
        effects.insert("my_ammo".to_string(), StateValue::Int(30));
        effects.insert("reloaded".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for ReloadAction {
    fn name(&self) -> &str {
        "reload"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        3.0
    }

    fn success_probability(&self, _world: &WorldState, _history: &ActionHistory) -> f32 {
        0.98 // Reload is very reliable
    }
}

/// Take Cover action: Move to defensive position
pub struct TakeCoverAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl TakeCoverAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("enemy_present".to_string(), StateValue::Bool(true));
        preconditions.insert("in_cover".to_string(), StateValue::Bool(false));

        let mut effects = BTreeMap::new();
        effects.insert("in_cover".to_string(), StateValue::Bool(true));
        effects.insert("protected".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for TakeCoverAction {
    fn name(&self) -> &str {
        "take_cover"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        2.0
    }

    fn state_cost_modifier(&self, world: &WorldState) -> f32 {
        // Taking cover becomes more urgent when health is low
        if let Some(StateValue::Int(health)) = world.get("my_health") {
            if *health < 30 {
                return 0.5; // Lower cost = higher priority
            } else if *health < 60 {
                return 0.8;
            }
        }
        1.0
    }
}

/// Heal action: Restore health using resources
pub struct HealAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl HealAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("has_medkit".to_string(), StateValue::Bool(true));
        preconditions.insert("my_health".to_string(), StateValue::IntRange(1, 80));

        let mut effects = BTreeMap::new();
        effects.insert("my_health".to_string(), StateValue::Int(100));
        effects.insert("healed".to_string(), StateValue::Bool(true));
        effects.insert("has_medkit".to_string(), StateValue::Bool(false));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for HealAction {
    fn name(&self) -> &str {
        "heal"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        4.0
    }

    fn state_cost_modifier(&self, world: &WorldState) -> f32 {
        // Healing becomes cheaper (higher priority) when critically wounded
        if let Some(StateValue::Int(health)) = world.get("my_health") {
            if *health < 20 {
                return 0.3; // Very urgent
            } else if *health < 50 {
                return 0.6;
            }
        }
        1.0
    }
}

/// Throw Smoke action: Deploy smoke for concealment
pub struct ThrowSmokeAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl ThrowSmokeAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("smoke_available".to_string(), StateValue::Bool(true));
        preconditions.insert(
            "smoke_cooldown".to_string(),
            StateValue::FloatApprox(0.0, 0.1),
        );

        let mut effects = BTreeMap::new();
        effects.insert("smoke_deployed".to_string(), StateValue::Bool(true));
        effects.insert("concealed".to_string(), StateValue::Bool(true));
        effects.insert(
            "smoke_cooldown".to_string(),
            StateValue::Float(super::OrderedFloat(8.0)),
        );

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for ThrowSmokeAction {
    fn name(&self) -> &str {
        "throw_smoke"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        4.0
    }
}

/// Retreat action: Fall back from enemy
pub struct RetreatAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl RetreatAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("enemy_present".to_string(), StateValue::Bool(true));
        preconditions.insert("enemy_distance".to_string(), StateValue::IntRange(0, 8));

        let mut effects = BTreeMap::new();
        effects.insert("enemy_distance".to_string(), StateValue::IntRange(10, 20));
        effects.insert("safe_distance".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for RetreatAction {
    fn name(&self) -> &str {
        "retreat"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        2.5
    }

    fn state_cost_modifier(&self, world: &WorldState) -> f32 {
        // Retreating becomes cheaper (more attractive) when health is low
        if let Some(StateValue::Int(health)) = world.get("my_health") {
            if *health < 25 {
                return 0.4; // Very attractive when critical
            } else if *health < 50 {
                return 0.7;
            }
        }
        1.0
    }
}

/// Revive action: Restore downed ally
pub struct ReviveAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl ReviveAction {
    pub fn new() -> Self {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("ally_downed".to_string(), StateValue::Bool(true));
        preconditions.insert("near_ally".to_string(), StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert("ally_downed".to_string(), StateValue::Bool(false));
        effects.insert("ally_revived".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for ReviveAction {
    fn name(&self) -> &str {
        "revive"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        5.0
    }
}

/// Wait/Scan action: Observe before acting
pub struct ScanAction {
    preconditions: BTreeMap<String, StateValue>,
    effects: BTreeMap<String, StateValue>,
}

impl ScanAction {
    pub fn new() -> Self {
        let preconditions = BTreeMap::new();

        let mut effects = BTreeMap::new();
        effects.insert("scanned".to_string(), StateValue::Bool(true));
        effects.insert("aware".to_string(), StateValue::Bool(true));

        Self {
            preconditions,
            effects,
        }
    }
}

impl Action for ScanAction {
    fn name(&self) -> &str {
        "scan"
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        1.0
    }
}

/// Register all actions into a GOAP planner
pub fn register_all_actions(planner: &mut crate::goap::AdvancedGOAP) {
    planner.add_action(Box::new(MoveToAction::new()));
    planner.add_action(Box::new(ApproachEnemyAction::new()));
    planner.add_action(Box::new(AttackAction::new()));
    planner.add_action(Box::new(CoverFireAction::new()));
    planner.add_action(Box::new(ReloadAction::new()));
    planner.add_action(Box::new(TakeCoverAction::new()));
    planner.add_action(Box::new(HealAction::new()));
    planner.add_action(Box::new(ThrowSmokeAction::new()));
    planner.add_action(Box::new(RetreatAction::new()));
    planner.add_action(Box::new(ReviveAction::new()));
    planner.add_action(Box::new(ScanAction::new()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attack_preconditions() {
        let action = AttackAction::new();
        assert_eq!(action.preconditions().len(), 3);
        assert!(action.preconditions().contains_key("has_ammo"));
        assert!(action.preconditions().contains_key("enemy_present"));
    }

    #[test]
    fn test_heal_state_cost_modifier() {
        let action = HealAction::new();

        let mut critical_health = WorldState::new();
        critical_health.set("my_health", StateValue::Int(15));
        assert!(action.state_cost_modifier(&critical_health) < 0.5);

        let mut full_health = WorldState::new();
        full_health.set("my_health", StateValue::Int(100));
        assert_eq!(action.state_cost_modifier(&full_health), 1.0);
    }

    #[test]
    fn test_retreat_becomes_attractive_when_wounded() {
        let action = RetreatAction::new();

        let mut critical = WorldState::new();
        critical.set("my_health", StateValue::Int(20));
        assert!(action.state_cost_modifier(&critical) < 0.5);
    }

    #[test]
    fn test_action_registration() {
        let mut planner = crate::goap::AdvancedGOAP::new();
        register_all_actions(&mut planner);

        assert_eq!(planner.action_count(), 11);
        let names = planner.action_names();
        assert!(names.contains(&"attack".to_string()));
        assert!(names.contains(&"heal".to_string()));
        assert!(names.contains(&"retreat".to_string()));
    }

    // =========================================================================
    // Mutation-Resistant: Exact name() and base_cost() for ALL actions
    // =========================================================================

    #[test]
    fn test_move_to_action_properties() {
        let action = MoveToAction::new();
        assert_eq!(action.name(), "move_to");
        assert_eq!(action.base_cost(), 2.0);
        assert!(action.preconditions().is_empty());
        assert!(action.effects().contains_key("moved"));
        assert!(action.effects().contains_key("position_changed"));
    }

    #[test]
    fn test_approach_enemy_action_properties() {
        let action = ApproachEnemyAction::new();
        assert_eq!(action.name(), "approach_enemy");
        assert_eq!(action.base_cost(), 2.5);
        assert!(action.preconditions().contains_key("enemy_present"));
        assert!(action.preconditions().contains_key("enemy_distance"));
        assert!(action.effects().contains_key("in_range"));
    }

    #[test]
    fn test_attack_action_properties() {
        let action = AttackAction::new();
        assert_eq!(action.name(), "attack");
        assert_eq!(action.base_cost(), 5.0);
        assert!(action.effects().contains_key("enemy_damaged"));
        assert!(action.effects().contains_key("ammo_spent"));
    }

    #[test]
    fn test_cover_fire_action_properties() {
        let action = CoverFireAction::new();
        assert_eq!(action.name(), "cover_fire");
        assert_eq!(action.base_cost(), 6.0);
        assert!(action.effects().contains_key("enemy_suppressed"));
        assert!(action.effects().contains_key("cover_provided"));
    }

    #[test]
    fn test_reload_action_properties() {
        let action = ReloadAction::new();
        assert_eq!(action.name(), "reload");
        assert_eq!(action.base_cost(), 3.0);
        // Reload has very high success probability
        let history = ActionHistory::new();
        let world = WorldState::new();
        assert_eq!(action.success_probability(&world, &history), 0.98);
    }

    #[test]
    fn test_take_cover_action_properties() {
        let action = TakeCoverAction::new();
        assert_eq!(action.name(), "take_cover");
        assert_eq!(action.base_cost(), 2.0);
        assert!(action.preconditions().contains_key("enemy_present"));
        assert!(action.preconditions().contains_key("in_cover"));
    }

    #[test]
    fn test_heal_action_properties() {
        let action = HealAction::new();
        assert_eq!(action.name(), "heal");
        assert_eq!(action.base_cost(), 4.0);
        assert!(action.preconditions().contains_key("has_medkit"));
        assert!(action.preconditions().contains_key("my_health"));
    }

    #[test]
    fn test_throw_smoke_action_properties() {
        let action = ThrowSmokeAction::new();
        assert_eq!(action.name(), "throw_smoke");
        assert_eq!(action.base_cost(), 4.0);
        assert!(action.effects().contains_key("smoke_deployed"));
        assert!(action.effects().contains_key("concealed"));
    }

    #[test]
    fn test_retreat_action_properties() {
        let action = RetreatAction::new();
        assert_eq!(action.name(), "retreat");
        assert_eq!(action.base_cost(), 2.5);
        assert!(action.effects().contains_key("safe_distance"));
    }

    #[test]
    fn test_revive_action_properties() {
        let action = ReviveAction::new();
        assert_eq!(action.name(), "revive");
        assert_eq!(action.base_cost(), 5.0);
        assert!(action.preconditions().contains_key("ally_downed"));
        assert!(action.preconditions().contains_key("near_ally"));
        assert!(action.effects().contains_key("ally_revived"));
    }

    #[test]
    fn test_scan_action_properties() {
        let action = ScanAction::new();
        assert_eq!(action.name(), "scan");
        assert_eq!(action.base_cost(), 1.0);
        assert!(action.preconditions().is_empty());
        assert!(action.effects().contains_key("scanned"));
        assert!(action.effects().contains_key("aware"));
    }

    // =========================================================================
    // Mutation-Resistant: success_probability and state_cost_modifier thresholds
    // =========================================================================

    #[test]
    fn test_approach_enemy_success_probability_low_health() {
        let action = ApproachEnemyAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(20)); // < 30
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        // Default 0.85 * 0.7 = 0.595
        assert!((prob - 0.595).abs() < 0.01);
    }

    #[test]
    fn test_approach_enemy_success_probability_healthy() {
        let action = ApproachEnemyAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(100));
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        assert_eq!(prob, 0.85);
    }

    #[test]
    fn test_attack_success_probability_low_health_low_ammo() {
        let action = AttackAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(30)); // < 40
        world.set("my_ammo", StateValue::Int(3)); // < 5
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        // 0.75 * 0.6 * 0.8 = 0.36
        assert!((prob - 0.36).abs() < 0.01);
    }

    #[test]
    fn test_take_cover_state_cost_modifier_critical() {
        let action = TakeCoverAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(20)); // < 30
        assert_eq!(action.state_cost_modifier(&world), 0.5);
    }

    #[test]
    fn test_take_cover_state_cost_modifier_moderate() {
        let action = TakeCoverAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(50)); // < 60
        assert_eq!(action.state_cost_modifier(&world), 0.8);
    }

    #[test]
    fn test_heal_state_cost_modifier_critical_exact() {
        let action = HealAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(15)); // < 20
        assert_eq!(action.state_cost_modifier(&world), 0.3);
    }

    #[test]
    fn test_heal_state_cost_modifier_moderate_exact() {
        let action = HealAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(40)); // < 50
        assert_eq!(action.state_cost_modifier(&world), 0.6);
    }

    #[test]
    fn test_retreat_state_cost_modifier_critical_exact() {
        let action = RetreatAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(20)); // < 25
        assert_eq!(action.state_cost_modifier(&world), 0.4);
    }

    #[test]
    fn test_retreat_state_cost_modifier_moderate_exact() {
        let action = RetreatAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(40)); // < 50
        assert_eq!(action.state_cost_modifier(&world), 0.7);
    }

    // =========================================================================
    // Round 2: Precondition/effect emptiness + boundary threshold tests
    // =========================================================================

    #[test]
    fn test_cover_fire_preconditions_non_empty() {
        let action = CoverFireAction::new();
        assert!(
            !action.preconditions().is_empty(),
            "CoverFire should have preconditions"
        );
        assert!(action.preconditions().contains_key("has_ammo"));
        assert!(action.preconditions().contains_key("enemy_present"));
        assert!(action.preconditions().contains_key("in_range"));
    }

    #[test]
    fn test_reload_preconditions_and_effects_non_empty() {
        let action = ReloadAction::new();
        assert!(
            !action.preconditions().is_empty(),
            "Reload should have preconditions"
        );
        assert!(!action.effects().is_empty(), "Reload should have effects");
        assert!(action.effects().contains_key("has_ammo"));
    }

    #[test]
    fn test_take_cover_effects_non_empty() {
        let action = TakeCoverAction::new();
        assert!(
            !action.effects().is_empty(),
            "TakeCover should have effects"
        );
        assert!(action.effects().contains_key("in_cover"));
        assert!(action.effects().contains_key("protected"));
    }

    #[test]
    fn test_heal_effects_non_empty() {
        let action = HealAction::new();
        assert!(!action.effects().is_empty(), "Heal should have effects");
        assert!(action.effects().contains_key("healed"));
    }

    #[test]
    fn test_throw_smoke_preconditions_non_empty() {
        let action = ThrowSmokeAction::new();
        assert!(
            !action.preconditions().is_empty(),
            "ThrowSmoke should have preconditions"
        );
        assert!(action.preconditions().contains_key("smoke_available"));
    }

    #[test]
    fn test_retreat_preconditions_non_empty() {
        let action = RetreatAction::new();
        assert!(
            !action.preconditions().is_empty(),
            "Retreat should have preconditions"
        );
        assert!(action.preconditions().contains_key("enemy_distance"));
    }

    // Boundary tests: exact threshold values kill `< → <=` mutations
    #[test]
    fn test_approach_enemy_health_at_exact_boundary_30() {
        let action = ApproachEnemyAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(30)); // Exactly 30: < 30 = false, <= 30 = true
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        // At health=30, < 30 is false → returns base (0.85), not base * 0.7
        assert_eq!(
            prob, 0.85,
            "At health=30, < 30 is false, should return base probability"
        );
    }

    #[test]
    fn test_attack_health_at_exact_boundary_40() {
        let action = AttackAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(40)); // Exactly 40: < 40 = false
        world.set("my_ammo", StateValue::Int(100)); // Plenty of ammo
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        // < 40 is false → modifier stays 1.0 → returns 0.75
        assert_eq!(
            prob, 0.75,
            "At health=40, < 40 is false, should return base probability"
        );
    }

    #[test]
    fn test_attack_ammo_at_exact_boundary_5() {
        let action = AttackAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(100));
        world.set("my_ammo", StateValue::Int(5)); // Exactly 5: < 5 = false
        let history = ActionHistory::new();
        let prob = action.success_probability(&world, &history);
        // < 5 is false → no ammo modifier → returns 0.75
        assert_eq!(
            prob, 0.75,
            "At ammo=5, < 5 is false, should return base probability"
        );
    }

    #[test]
    fn test_take_cover_health_at_exact_boundary_30() {
        let action = TakeCoverAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(30)); // < 30 = false
                                                     // < 60 = true → returns 0.8
        assert_eq!(
            action.state_cost_modifier(&world),
            0.8,
            "At health=30, < 30 = false, should fall to < 60 check → 0.8"
        );
    }

    #[test]
    fn test_take_cover_health_at_exact_boundary_60() {
        let action = TakeCoverAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(60)); // < 60 = false
        assert_eq!(
            action.state_cost_modifier(&world),
            1.0,
            "At health=60, < 60 = false, should return default 1.0"
        );
    }

    #[test]
    fn test_heal_health_at_exact_boundary_20() {
        let action = HealAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(20)); // < 20 = false
                                                     // < 50 = true → returns 0.6
        assert_eq!(
            action.state_cost_modifier(&world),
            0.6,
            "At health=20, < 20 = false, should fall to < 50 check → 0.6"
        );
    }

    #[test]
    fn test_heal_health_at_exact_boundary_50() {
        let action = HealAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(50)); // < 50 = false
        assert_eq!(
            action.state_cost_modifier(&world),
            1.0,
            "At health=50, < 50 = false, should return default 1.0"
        );
    }

    #[test]
    fn test_retreat_health_at_exact_boundary_25() {
        let action = RetreatAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(25)); // < 25 = false
                                                     // < 50 = true → returns 0.7
        assert_eq!(
            action.state_cost_modifier(&world),
            0.7,
            "At health=25, < 25 = false, should fall to < 50 check → 0.7"
        );
    }

    #[test]
    fn test_retreat_health_at_exact_boundary_50() {
        let action = RetreatAction::new();
        let mut world = WorldState::new();
        world.set("my_health", StateValue::Int(50)); // < 50 = false
        assert_eq!(
            action.state_cost_modifier(&world),
            1.0,
            "At health=50, < 50 = false, should return default 1.0"
        );
    }
}
