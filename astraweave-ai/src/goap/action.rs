use super::history::ActionHistory;
use super::{StateValue, WorldState};
use std::collections::BTreeMap;

/// Trait for GOAP actions with dynamic cost and risk assessment
pub trait Action: Send + Sync {
    /// Unique name for this action
    fn name(&self) -> &str;

    /// Preconditions that must be satisfied for this action to execute
    fn preconditions(&self) -> &BTreeMap<String, StateValue>;

    /// Effects that this action applies to the world state
    fn effects(&self) -> &BTreeMap<String, StateValue>;

    /// Base cost of executing this action (without history adjustment)
    fn base_cost(&self) -> f32;

    /// Calculate dynamic cost based on current world state and history
    fn calculate_cost(&self, _world: &WorldState, history: &ActionHistory) -> f32 {
        let mut cost = self.base_cost();

        // Apply historical performance adjustments
        if let Some(stats) = history.get_action_stats(self.name()) {
            // Penalize frequently failed actions (up to +10.0 cost)
            let failure_penalty = stats.failure_rate() * 10.0;

            // Reward consistently successful actions (up to -2.0 cost)
            let success_bonus = stats.success_rate() * -2.0;

            cost += failure_penalty + success_bonus;
        }

        cost.max(0.1) // Ensure cost never goes to zero or negative
    }

    /// Check if action can execute in current state (preconditions satisfied)
    fn can_execute(&self, world: &WorldState) -> bool {
        world.satisfies(self.preconditions())
    }

    /// Estimate probability of success (0.0 to 1.0)
    /// Used for risk-aware planning
    fn success_probability(&self, _world: &WorldState, history: &ActionHistory) -> f32 {
        history
            .get_action_stats(self.name())
            .map(|stats| stats.success_rate())
            .unwrap_or(0.8) // Default 80% success for unknown actions
    }

    /// Optional: Context-specific cost modifier based on world state
    /// Override this to implement state-dependent costs
    fn state_cost_modifier(&self, _world: &WorldState) -> f32 {
        1.0 // Default: no modification
    }

    /// Optional: Duration estimate for this action (in seconds)
    /// Used for time-constrained planning
    fn estimated_duration(&self, history: &ActionHistory) -> f32 {
        history
            .get_action_stats(self.name())
            .map(|stats| stats.avg_duration)
            .unwrap_or(1.0) // Default 1 second
    }
}

/// Helper struct for actions with static preconditions/effects
/// Useful for simple actions that don't need complex logic
#[derive(Clone)]
pub struct SimpleAction {
    pub name: String,
    pub preconditions: BTreeMap<String, StateValue>,
    pub effects: BTreeMap<String, StateValue>,
    pub cost: f32,
}

impl SimpleAction {
    pub fn new(
        name: impl Into<String>,
        preconditions: BTreeMap<String, StateValue>,
        effects: BTreeMap<String, StateValue>,
        cost: f32,
    ) -> Self {
        Self {
            name: name.into(),
            preconditions,
            effects,
            cost,
        }
    }
}

impl Action for SimpleAction {
    fn name(&self) -> &str {
        &self.name
    }

    fn preconditions(&self) -> &BTreeMap<String, StateValue> {
        &self.preconditions
    }

    fn effects(&self) -> &BTreeMap<String, StateValue> {
        &self.effects
    }

    fn base_cost(&self) -> f32 {
        self.cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_action() {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("has_weapon".to_string(), StateValue::Bool(true));

        let mut effects = BTreeMap::new();
        effects.insert("weapon_equipped".to_string(), StateValue::Bool(true));

        let action = SimpleAction::new("equip_weapon", preconditions, effects, 1.0);

        assert_eq!(action.name(), "equip_weapon");
        assert_eq!(action.base_cost(), 1.0);
    }

    #[test]
    fn test_action_can_execute() {
        let mut preconditions = BTreeMap::new();
        preconditions.insert("ammo".to_string(), StateValue::IntRange(1, 100));

        let mut effects = BTreeMap::new();
        effects.insert("target_hit".to_string(), StateValue::Bool(true));

        let action = SimpleAction::new("shoot", preconditions, effects, 2.0);

        let mut world = WorldState::new();
        world.set("ammo", StateValue::Int(10));
        assert!(action.can_execute(&world));

        let mut world_no_ammo = WorldState::new();
        world_no_ammo.set("ammo", StateValue::Int(0));
        assert!(!action.can_execute(&world_no_ammo));
    }

    #[test]
    fn test_calculate_cost_with_history() {
        let action = SimpleAction::new("attack", BTreeMap::new(), BTreeMap::new(), 5.0);

        let mut history = ActionHistory::new();

        // Record some failures
        history.record_failure("attack");
        history.record_failure("attack");
        history.record_success("attack", 1.0);

        let world = WorldState::new();
        let cost = action.calculate_cost(&world, &history);

        // Base cost 5.0 + failure penalty (0.66 * 10) - success bonus (0.33 * 2)
        // Should be higher than base due to failures
        assert!(cost > action.base_cost());
    }

    #[test]
    fn test_success_probability_default() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 1.0);
        let history = ActionHistory::new();
        let world = WorldState::new();

        assert_eq!(action.success_probability(&world, &history), 0.8);
    }

    #[test]
    fn test_success_probability_from_history() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 1.0);
        let mut history = ActionHistory::new();
        let world = WorldState::new();

        history.record_success("test", 1.0);
        history.record_success("test", 1.0);
        history.record_failure("test");

        // 2 successes out of 3 = 0.666...
        let prob = action.success_probability(&world, &history);
        assert!((prob - 0.666666).abs() < 0.01);
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    /// Kills line 26: `failure_rate() * 10.0` → `+`
    /// Kills line 29: `success_rate() * -2.0` → `+`, `/`, and `delete -`
    /// Kills line 31: `failure_penalty + success_bonus` → `-`, and `+= → *=`
    #[test]
    fn test_calculate_cost_exact_values_all_success() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 5.0);
        let world = WorldState::new();
        let mut history = ActionHistory::new();

        // Record 10 successes (failure_rate=0.0, success_rate=1.0)
        for _ in 0..10 {
            history.record_success("test", 1.0);
        }

        let cost = action.calculate_cost(&world, &history);
        // penalty = 0.0 * 10.0 = 0.0, bonus = 1.0 * -2.0 = -2.0
        // cost = 5.0 + 0.0 + (-2.0) = 3.0
        assert!(
            (cost - 3.0).abs() < 0.01,
            "Cost with all successes should be 3.0, got {}",
            cost
        );
    }

    /// Kills same lines from the failure perspective
    #[test]
    fn test_calculate_cost_exact_values_all_failures() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 5.0);
        let world = WorldState::new();
        let mut history = ActionHistory::new();

        // Record 10 failures (failure_rate=1.0, success_rate=0.0)
        for _ in 0..10 {
            history.record_failure("test");
        }

        let cost = action.calculate_cost(&world, &history);
        // penalty = 1.0 * 10.0 = 10.0, bonus = 0.0 * -2.0 = 0.0
        // cost = 5.0 + 10.0 + 0.0 = 15.0
        assert!(
            (cost - 15.0).abs() < 0.01,
            "Cost with all failures should be 15.0, got {}",
            cost
        );
    }

    /// Kills line 54: `state_cost_modifier -> f32` replaced with 0.0 / -1.0
    #[test]
    fn test_state_cost_modifier_default() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 1.0);
        let world = WorldState::new();
        let modifier = action.state_cost_modifier(&world);
        assert!(
            (modifier - 1.0).abs() < f32::EPSILON,
            "Default state_cost_modifier should be 1.0, got {}",
            modifier
        );
    }

    /// Kills line 60: `estimated_duration -> f32` replaced with 0.0 / 1.0 / -1.0
    #[test]
    fn test_estimated_duration_default_and_with_history() {
        let action = SimpleAction::new("test", BTreeMap::new(), BTreeMap::new(), 1.0);
        let history = ActionHistory::new();

        // No history → default 1.0
        let dur = action.estimated_duration(&history);
        assert!(
            (dur - 1.0).abs() < f32::EPSILON,
            "Default estimated_duration should be 1.0"
        );

        // With history → avg_duration from stats
        let mut history2 = ActionHistory::new();
        history2.record_success("test", 2.0);
        history2.record_success("test", 4.0);
        let dur2 = action.estimated_duration(&history2);
        // avg_duration = (2.0 + 4.0) / 2 = 3.0
        assert!(
            (dur2 - 3.0).abs() < 0.01,
            "estimated_duration with history should be avg ~3.0, got {}",
            dur2
        );
    }
}
