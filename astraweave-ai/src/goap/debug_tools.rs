use super::{Action, Goal, StateValue, WorldState};
use std::collections::BTreeMap;

/// State difference between two world states
#[derive(Debug, Clone)]
pub struct StateDiff {
    pub added: BTreeMap<String, StateValue>,
    pub removed: BTreeMap<String, StateValue>,
    pub changed: Vec<StateChange>,
}

/// A single state change
#[derive(Debug, Clone)]
pub struct StateChange {
    pub key: String,
    pub old_value: StateValue,
    pub new_value: StateValue,
}

/// Explanation for why an action was chosen
#[derive(Debug, Clone)]
pub struct Explanation {
    pub action_name: String,
    pub reason: String,
    pub preconditions_met: Vec<String>,
    pub effects_applied: Vec<String>,
    pub cost: f32,
    pub risk: f32,
    pub alternatives_considered: Vec<String>,
}

/// Progress report for a goal
#[derive(Debug, Clone)]
pub struct ProgressReport {
    pub goal_name: String,
    pub progress: f32, // 0.0 - 1.0
    pub satisfied_conditions: Vec<String>,
    pub unsatisfied_conditions: Vec<String>,
    pub estimated_actions_remaining: usize,
}

/// Interactive plan debugger
pub struct PlanDebugger {
    plan: Vec<String>,
    current_step: usize,
    state_history: Vec<WorldState>,
    actions: Vec<Box<dyn Action>>,
}

impl PlanDebugger {
    /// Create a new debugger
    pub fn new(plan: Vec<String>, start_state: WorldState, actions: Vec<Box<dyn Action>>) -> Self {
        Self {
            plan,
            current_step: 0,
            state_history: vec![start_state],
            actions,
        }
    }

    /// Get current step number
    pub fn current_step(&self) -> usize {
        self.current_step
    }

    /// Get total number of steps
    pub fn total_steps(&self) -> usize {
        self.plan.len()
    }

    /// Check if at beginning
    pub fn at_start(&self) -> bool {
        self.current_step == 0
    }

    /// Check if at end
    pub fn at_end(&self) -> bool {
        self.current_step >= self.plan.len()
    }

    /// Get current action name
    pub fn current_action(&self) -> Option<&str> {
        self.plan.get(self.current_step).map(|s| s.as_str())
    }

    /// Get current world state
    pub fn current_state(&self) -> &WorldState {
        self.state_history
            .last()
            .expect("state_history must not be empty")
    }

    /// Step forward one action
    pub fn step_forward(&mut self) -> Result<(), String> {
        if self.at_end() {
            return Err("Already at end of plan".to_string());
        }

        let action_name = &self.plan[self.current_step];
        let action = self
            .actions
            .iter()
            .find(|a| a.name() == action_name)
            .ok_or_else(|| format!("Action '{}' not found", action_name))?;

        let current_state = self.current_state();

        if !action.can_execute(current_state) {
            return Err(format!("Action '{}' preconditions not met", action_name));
        }

        // Apply effects
        let mut new_state = current_state.clone();
        new_state.apply_effects(action.effects());

        self.state_history.push(new_state);
        self.current_step += 1;

        Ok(())
    }

    /// Step backward one action
    pub fn step_backward(&mut self) -> Result<(), String> {
        if self.at_start() {
            return Err("Already at start of plan".to_string());
        }

        self.state_history.pop();
        self.current_step -= 1;

        Ok(())
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        let start_state = self.state_history[0].clone();
        self.state_history = vec![start_state];
        self.current_step = 0;
    }

    /// Get state difference from previous step
    pub fn get_state_diff(&self) -> Option<StateDiff> {
        if self.current_step == 0 {
            return None;
        }

        let prev_state = &self.state_history[self.current_step - 1];
        let curr_state = &self.state_history[self.current_step];

        Some(Self::diff_states(prev_state, curr_state))
    }

    /// Calculate difference between two states
    fn diff_states(old_state: &WorldState, new_state: &WorldState) -> StateDiff {
        let mut added = BTreeMap::new();
        let mut removed = BTreeMap::new();
        let mut changed = Vec::new();

        // Find added and changed
        for (key, new_value) in new_state.iter() {
            if let Some(old_value) = old_state.get(key) {
                if old_value != new_value {
                    changed.push(StateChange {
                        key: key.clone(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                }
            } else {
                added.insert(key.clone(), new_value.clone());
            }
        }

        // Find removed
        for (key, old_value) in old_state.iter() {
            if !new_state.contains_key(key) {
                removed.insert(key.clone(), old_value.clone());
            }
        }

        StateDiff {
            added,
            removed,
            changed,
        }
    }

    /// Explain why current action was chosen
    pub fn explain_action(&self, action_name: &str) -> Option<Explanation> {
        let action = self.actions.iter().find(|a| a.name() == action_name)?;

        let state = self.current_state();

        let mut preconditions_met = Vec::new();
        for (key, value) in action.preconditions() {
            if let Some(current_value) = state.get(&key) {
                if current_value.satisfies(&value) {
                    preconditions_met.push(format!("{}={:?}", key, current_value));
                }
            }
        }

        let mut effects_applied = Vec::new();
        for (key, value) in action.effects() {
            effects_applied.push(format!("{}={:?}", key, value));
        }

        // Find alternatives that were not chosen
        let mut alternatives_considered = Vec::new();
        for other_action in &self.actions {
            if other_action.name() != action_name && other_action.can_execute(state) {
                alternatives_considered.push(other_action.name().to_string());
            }
        }

        Some(Explanation {
            action_name: action_name.to_string(),
            reason: format!(
                "Action satisfies {} preconditions and applies {} effects",
                preconditions_met.len(),
                effects_applied.len()
            ),
            preconditions_met,
            effects_applied,
            cost: action.base_cost(),
            risk: 0.0, // Would need history for accurate risk
            alternatives_considered,
        })
    }

    /// Check progress toward a goal
    pub fn check_goal_progress(&self, goal: &Goal) -> ProgressReport {
        let current_state = self.current_state();
        let progress = goal.progress(current_state);

        let mut satisfied = Vec::new();
        let mut unsatisfied = Vec::new();

        for (key, target_value) in &goal.desired_state {
            if let Some(current_value) = current_state.get(key) {
                if current_value.satisfies(target_value) {
                    satisfied.push(format!("{}={:?}", key, current_value));
                } else {
                    unsatisfied.push(format!(
                        "{}={:?} (need {:?})",
                        key, current_value, target_value
                    ));
                }
            } else {
                unsatisfied.push(format!("{} (missing)", key));
            }
        }

        let remaining_steps = self.plan.len() - self.current_step;

        ProgressReport {
            goal_name: goal.name.clone(),
            progress,
            satisfied_conditions: satisfied,
            unsatisfied_conditions: unsatisfied,
            estimated_actions_remaining: remaining_steps,
        }
    }

    /// Get all actions in plan
    pub fn get_plan(&self) -> &[String] {
        &self.plan
    }

    /// Get action at specific step
    pub fn get_action_at_step(&self, step: usize) -> Option<&str> {
        self.plan.get(step).map(|s| s.as_str())
    }

    /// Jump to specific step
    pub fn jump_to_step(&mut self, step: usize) -> Result<(), String> {
        if step > self.plan.len() {
            return Err(format!(
                "Step {} is beyond plan length {}",
                step,
                self.plan.len()
            ));
        }

        // Reset to start
        self.reset();

        // Step forward to target
        while self.current_step < step {
            self.step_forward()?;
        }

        Ok(())
    }

    /// Get human-readable current state
    pub fn format_current_state(&self) -> String {
        let state = self.current_state();
        let mut output = String::new();

        output.push_str(&format!(
            "=== World State at Step {} ===\n",
            self.current_step
        ));

        let mut keys: Vec<_> = state.iter().map(|(k, _)| k).collect();
        keys.sort();

        for key in keys {
            if let Some(value) = state.get(key) {
                output.push_str(&format!("  {} = {:?}\n", key, value));
            }
        }

        output
    }

    /// Get human-readable diff
    pub fn format_state_diff(&self) -> String {
        let Some(diff) = self.get_state_diff() else {
            return "No previous state to compare".to_string();
        };

        let mut output = String::new();

        output.push_str("=== State Changes ===\n");

        if !diff.added.is_empty() {
            output.push_str("\nAdded:\n");
            for (key, value) in &diff.added {
                output.push_str(&format!("  + {} = {:?}\n", key, value));
            }
        }

        if !diff.removed.is_empty() {
            output.push_str("\nRemoved:\n");
            for (key, value) in &diff.removed {
                output.push_str(&format!("  - {} = {:?}\n", key, value));
            }
        }

        if !diff.changed.is_empty() {
            output.push_str("\nChanged:\n");
            for change in &diff.changed {
                output.push_str(&format!(
                    "  ~ {} = {:?} -> {:?}\n",
                    change.key, change.old_value, change.new_value
                ));
            }
        }

        if diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty() {
            output.push_str("  (no changes)\n");
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goap::SimpleAction;
    use std::collections::BTreeMap;

    fn create_test_action(name: &str, effects: Vec<(&str, StateValue)>) -> Box<dyn Action> {
        let mut effects_map = BTreeMap::new();
        for (key, value) in effects {
            effects_map.insert(key.to_string(), value);
        }
        Box::new(SimpleAction::new(name, BTreeMap::new(), effects_map, 1.0))
    }

    #[test]
    fn test_debugger_creation() {
        let plan = vec!["action1".to_string(), "action2".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action("action1", vec![])];

        let debugger = PlanDebugger::new(plan, start, actions);

        assert_eq!(debugger.current_step(), 0);
        assert_eq!(debugger.total_steps(), 2);
        assert!(debugger.at_start());
        assert!(!debugger.at_end());
    }

    #[test]
    fn test_step_forward() {
        let plan = vec!["action1".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action(
            "action1",
            vec![("flag", StateValue::Bool(true))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        assert!(debugger.step_forward().is_ok());
        assert_eq!(debugger.current_step(), 1);
        assert!(debugger.at_end());

        // Check that state was updated
        let state = debugger.current_state();
        assert_eq!(state.get("flag"), Some(&StateValue::Bool(true)));
    }

    #[test]
    fn test_step_backward() {
        let plan = vec!["action1".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action(
            "action1",
            vec![("flag", StateValue::Bool(true))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        debugger.step_forward().unwrap();
        assert_eq!(debugger.current_step(), 1);

        debugger.step_backward().unwrap();
        assert_eq!(debugger.current_step(), 0);

        // Should be back to original state
        let state = debugger.current_state();
        assert_eq!(state.get("flag"), None);
    }

    #[test]
    fn test_state_diff() {
        let plan = vec!["action1".to_string()];
        let mut start = WorldState::new();
        start.set("existing", StateValue::Int(10));

        let actions = vec![create_test_action(
            "action1",
            vec![
                ("existing", StateValue::Int(20)),
                ("new", StateValue::Bool(true)),
            ],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let diff = debugger.get_state_diff().unwrap();

        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains_key("new"));

        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].key, "existing");
    }

    #[test]
    fn test_reset() {
        let plan = vec!["action1".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action(
            "action1",
            vec![("flag", StateValue::Bool(true))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        debugger.step_forward().unwrap();
        assert_eq!(debugger.current_step(), 1);

        debugger.reset();
        assert_eq!(debugger.current_step(), 0);
        assert!(debugger.current_state().get("flag").is_none());
    }

    #[test]
    fn test_jump_to_step() {
        let plan = vec![
            "action1".to_string(),
            "action2".to_string(),
            "action3".to_string(),
        ];
        let start = WorldState::new();
        let actions = vec![
            create_test_action("action1", vec![("step", StateValue::Int(1))]),
            create_test_action("action2", vec![("step", StateValue::Int(2))]),
            create_test_action("action3", vec![("step", StateValue::Int(3))]),
        ];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        debugger.jump_to_step(2).unwrap();
        assert_eq!(debugger.current_step(), 2);
        assert_eq!(
            debugger.current_state().get("step"),
            Some(&StateValue::Int(2))
        );
    }

    #[test]
    fn test_goal_progress() {
        let plan = vec!["action1".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action(
            "action1",
            vec![("goal_met", StateValue::Bool(true))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        let mut desired = BTreeMap::new();
        desired.insert("goal_met".to_string(), StateValue::Bool(true));
        let goal = Goal::new("test_goal", desired);

        // Before action
        let progress1 = debugger.check_goal_progress(&goal);
        assert_eq!(progress1.progress, 0.0);
        assert_eq!(progress1.unsatisfied_conditions.len(), 1);

        // After action
        debugger.step_forward().unwrap();
        let progress2 = debugger.check_goal_progress(&goal);
        assert_eq!(progress2.progress, 1.0);
        assert_eq!(progress2.satisfied_conditions.len(), 1);
    }

    // ── mutation-killing tests ──

    fn create_test_action_with_preconditions(
        name: &str,
        preconditions: Vec<(&str, StateValue)>,
        effects: Vec<(&str, StateValue)>,
        cost: f32,
    ) -> Box<dyn Action> {
        let mut pre_map = BTreeMap::new();
        for (key, value) in preconditions {
            pre_map.insert(key.to_string(), value);
        }
        let mut eff_map = BTreeMap::new();
        for (key, value) in effects {
            eff_map.insert(key.to_string(), value);
        }
        Box::new(SimpleAction::new(name, pre_map, eff_map, cost))
    }

    #[test]
    fn test_current_action_returns_correct_name() {
        let plan = vec!["alpha".to_string(), "beta".to_string()];
        let start = WorldState::new();
        let actions = vec![
            create_test_action("alpha", vec![("a", StateValue::Bool(true))]),
            create_test_action("beta", vec![("b", StateValue::Bool(true))]),
        ];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        // At step 0, current action is "alpha"
        assert_eq!(debugger.current_action(), Some("alpha"));

        debugger.step_forward().unwrap();
        assert_eq!(debugger.current_action(), Some("beta"));

        debugger.step_forward().unwrap();
        // At end, no current action
        assert_eq!(debugger.current_action(), None);
    }

    #[test]
    fn test_get_plan_returns_all_actions() {
        let plan = vec!["act1".to_string(), "act2".to_string(), "act3".to_string()];
        let start = WorldState::new();
        let debugger = PlanDebugger::new(plan.clone(), start, vec![]);

        let returned_plan = debugger.get_plan();
        assert_eq!(returned_plan.len(), 3);
        assert_eq!(returned_plan[0], "act1");
        assert_eq!(returned_plan[1], "act2");
        assert_eq!(returned_plan[2], "act3");
    }

    #[test]
    fn test_get_action_at_step_valid_and_invalid() {
        let plan = vec!["first".to_string(), "second".to_string()];
        let start = WorldState::new();
        let debugger = PlanDebugger::new(plan, start, vec![]);

        assert_eq!(debugger.get_action_at_step(0), Some("first"));
        assert_eq!(debugger.get_action_at_step(1), Some("second"));
        assert_eq!(debugger.get_action_at_step(2), None);
        assert_eq!(debugger.get_action_at_step(999), None);
    }

    #[test]
    fn test_jump_to_step_boundary() {
        let plan = vec!["a".to_string(), "b".to_string()];
        let start = WorldState::new();
        let actions = vec![
            create_test_action("a", vec![("x", StateValue::Int(1))]),
            create_test_action("b", vec![("x", StateValue::Int(2))]),
        ];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        // Jump to exact plan length should succeed (step past all actions)
        assert!(debugger.jump_to_step(2).is_ok());
        assert_eq!(debugger.current_step(), 2);

        // Jump beyond plan length should fail
        assert!(debugger.jump_to_step(3).is_err());
    }

    #[test]
    fn test_explain_action_preconditions_and_effects() {
        let mut start = WorldState::new();
        start.set("armed", StateValue::Bool(true));
        start.set("has_ammo", StateValue::Bool(true));

        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action_with_preconditions(
                "fire",
                vec![
                    ("armed", StateValue::Bool(true)),
                    ("has_ammo", StateValue::Bool(true)),
                ],
                vec![("enemy_hit", StateValue::Bool(true))],
                5.0,
            ),
            create_test_action_with_preconditions(
                "move",
                vec![],
                vec![("moved", StateValue::Bool(true))],
                2.0,
            ),
        ];

        let plan = vec!["fire".to_string()];
        let debugger = PlanDebugger::new(plan, start, actions);

        let explanation = debugger.explain_action("fire").unwrap();

        assert_eq!(explanation.action_name, "fire");
        assert_eq!(explanation.preconditions_met.len(), 2);
        assert_eq!(explanation.effects_applied.len(), 1);
        assert_eq!(explanation.cost, 5.0);
        assert_eq!(explanation.risk, 0.0);
        // "move" is executable, should be in alternatives
        assert!(explanation
            .alternatives_considered
            .contains(&"move".to_string()));
        // "fire" itself should NOT be in alternatives
        assert!(!explanation
            .alternatives_considered
            .contains(&"fire".to_string()));
        // reason string should contain the counts
        assert!(explanation.reason.contains("2 preconditions"));
        assert!(explanation.reason.contains("1 effects"));
    }

    #[test]
    fn test_explain_action_unknown_returns_none() {
        let plan = vec!["a".to_string()];
        let start = WorldState::new();
        let actions = vec![create_test_action("a", vec![])];
        let debugger = PlanDebugger::new(plan, start, actions);

        assert!(debugger.explain_action("nonexistent").is_none());
    }

    #[test]
    fn test_check_goal_progress_remaining_steps() {
        let plan = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let start = WorldState::new();
        let actions = vec![
            create_test_action("a", vec![("x", StateValue::Int(1))]),
            create_test_action("b", vec![("x", StateValue::Int(2))]),
            create_test_action("c", vec![("x", StateValue::Int(3))]),
        ];

        let mut debugger = PlanDebugger::new(plan, start, actions);

        let mut desired = BTreeMap::new();
        desired.insert("x".to_string(), StateValue::Int(3));
        let goal = Goal::new("get_x_3", desired);

        // At step 0: remaining = 3 - 0 = 3
        let report0 = debugger.check_goal_progress(&goal);
        assert_eq!(report0.estimated_actions_remaining, 3);
        assert_eq!(report0.goal_name, "get_x_3");

        debugger.step_forward().unwrap();
        // At step 1: remaining = 3 - 1 = 2
        let report1 = debugger.check_goal_progress(&goal);
        assert_eq!(report1.estimated_actions_remaining, 2);
    }

    #[test]
    fn test_diff_states_detects_change() {
        // Test that changed values trigger a 'changed' entry (not just added)
        let mut old = WorldState::new();
        old.set("hp", StateValue::Int(100));
        old.set("ammo", StateValue::Int(10));

        let mut new = WorldState::new();
        new.set("hp", StateValue::Int(50)); // changed
        new.set("ammo", StateValue::Int(10)); // same — should NOT appear

        let diff = PlanDebugger::diff_states(&old, &new);

        // "hp" changed from 100 to 50
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].key, "hp");
        assert_eq!(diff.changed[0].old_value, StateValue::Int(100));
        assert_eq!(diff.changed[0].new_value, StateValue::Int(50));

        // No added or removed
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_format_current_state_content() {
        let mut start = WorldState::new();
        start.set("health", StateValue::Int(100));

        let plan = vec!["a".to_string()];
        let actions = vec![create_test_action(
            "a",
            vec![("health", StateValue::Int(50))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let output = debugger.format_current_state();
        assert!(output.contains("Step 1"), "should contain step number");
        assert!(output.contains("health"), "should contain state key");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_state_diff_no_previous() {
        let start = WorldState::new();
        let plan = vec!["a".to_string()];
        let debugger = PlanDebugger::new(plan, start, vec![]);

        let output = debugger.format_state_diff();
        assert!(output.contains("No previous state"));
    }

    #[test]
    fn test_format_state_diff_with_changes() {
        let mut start = WorldState::new();
        start.set("hp", StateValue::Int(100));

        let plan = vec!["a".to_string()];
        let actions = vec![create_test_action(
            "a",
            vec![
                ("hp", StateValue::Int(50)),      // changed
                ("flag", StateValue::Bool(true)), // added
            ],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let output = debugger.format_state_diff();
        assert!(output.contains("Added"), "should have Added section");
        assert!(output.contains("Changed"), "should have Changed section");
        assert!(output.contains("+"), "should have + marker for added");
        assert!(output.contains("~"), "should have ~ marker for changed");
    }

    #[test]
    fn test_format_state_diff_no_changes() {
        // Action without effects → no state changes
        let start = WorldState::new();
        let plan = vec!["noop".to_string()];
        let actions = vec![create_test_action("noop", vec![])];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let output = debugger.format_state_diff();
        assert!(output.contains("no changes"));
    }

    #[test]
    fn test_format_state_diff_added_only_no_false_no_changes() {
        // Start empty, action ADDS a key → should NOT say "no changes"
        let start = WorldState::new();
        let plan = vec!["add_key".to_string()];
        let actions = vec![create_test_action(
            "add_key",
            vec![("new_flag", StateValue::Bool(true))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let output = debugger.format_state_diff();
        assert!(output.contains("Added"), "should show Added section");
        assert!(
            !output.contains("no changes"),
            "should NOT say 'no changes' when items were added"
        );
    }

    #[test]
    fn test_format_state_diff_changed_only_no_false_no_changes() {
        // Start with key, action changes it → should NOT say "no changes"
        let mut start = WorldState::new();
        start.set("x", StateValue::Int(1));

        let plan = vec!["change_x".to_string()];
        let actions = vec![create_test_action(
            "change_x",
            vec![("x", StateValue::Int(99))],
        )];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        let output = debugger.format_state_diff();
        assert!(output.contains("Changed"), "should show Changed section");
        assert!(
            !output.contains("no changes"),
            "should NOT say 'no changes' when items were changed"
        );
    }

    #[test]
    fn test_format_state_diff_with_removed_key() {
        // Directly manipulate state_history to test removed-key formatting
        let mut start = WorldState::new();
        start.set("will_remove", StateValue::Bool(true));
        start.set("stays", StateValue::Int(5));

        let plan = vec!["noop".to_string()];
        let actions = vec![create_test_action("noop", vec![])];

        let mut debugger = PlanDebugger::new(plan, start, actions);
        debugger.step_forward().unwrap();

        // Manually replace the post-step state to simulate removal
        let mut modified = WorldState::new();
        modified.set("stays", StateValue::Int(5));
        // "will_remove" is absent → should appear as removed
        debugger.state_history[1] = modified;

        let output = debugger.format_state_diff();
        assert!(output.contains("Removed"), "should show Removed section");
        assert!(output.contains("-"), "should have - marker for removed");
        assert!(
            !output.contains("no changes"),
            "should NOT say 'no changes'"
        );
    }
}
