use super::{Action, WorldState};
use std::collections::HashSet;

/// Represents a conflict detected in a plan
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Conflict {
    /// Two actions try to set the same state variable to different values
    StateConflict {
        action1: String,
        action2: String,
        variable: String,
    },
    /// An action's preconditions are not met based on previous actions
    PreconditionViolation {
        action: String,
        missing_condition: String,
    },
    /// Actions have incompatible requirements (e.g., can't move and attack simultaneously)
    IncompatibleActions {
        action1: String,
        action2: String,
        reason: String,
    },
}

/// Error types for plan stitching
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
#[must_use]
pub enum StitchError {
    /// Plans have unresolvable conflicts
    ConflictDetected(Vec<Conflict>),
    /// No valid way to merge plans
    NoValidMerge,
    /// Plans are empty
    EmptyPlans,
}

/// Plan stitcher for combining multiple sub-plans
pub struct PlanStitcher;

impl PlanStitcher {
    /// Merge plans sequentially (plan1, then plan2, etc.)
    pub fn merge_sequential(plans: Vec<Vec<String>>) -> Result<Vec<String>, StitchError> {
        if plans.is_empty() {
            return Err(StitchError::EmptyPlans);
        }

        let mut result = Vec::new();
        for plan in plans {
            result.extend(plan);
        }

        Ok(result)
    }

    /// Merge plans with interleaving based on priorities
    /// Higher priority plans get more actions at the front
    pub fn merge_interleaved(
        plans: Vec<Vec<String>>,
        priorities: Vec<f32>,
    ) -> Result<Vec<String>, StitchError> {
        if plans.is_empty() {
            return Err(StitchError::EmptyPlans);
        }

        if plans.len() != priorities.len() {
            return Err(StitchError::NoValidMerge);
        }

        // Create working copies with indices
        let mut plan_iters: Vec<(usize, Vec<String>, f32)> = plans
            .into_iter()
            .enumerate()
            .zip(priorities)
            .map(|((idx, plan), priority)| (idx, plan, priority))
            .collect();

        let mut result = Vec::new();

        // Interleave actions based on priority
        while !plan_iters.is_empty() {
            // Sort by priority (descending)
            plan_iters.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

            // Take one action from the highest priority plan
            if let Some((_idx, ref mut plan, _priority)) = plan_iters.first_mut() {
                if !plan.is_empty() {
                    result.push(plan.remove(0));
                }

                // Remove empty plans
                if plan.is_empty() {
                    plan_iters.remove(0);
                }
            }
        }

        Ok(result)
    }

    /// Detect conflicts in a plan given a starting world state
    pub fn detect_conflicts(
        plan: &[String],
        actions: &[Box<dyn Action>],
        start_state: &WorldState,
    ) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        let mut simulated_state = start_state.clone();

        for (i, action_name) in plan.iter().enumerate() {
            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                // Check preconditions
                if !action.can_execute(&simulated_state) {
                    // Find which conditions are missing
                    for (key, value) in action.preconditions() {
                        if let Some(current_value) = simulated_state.get(&key) {
                            if !current_value.satisfies(&value) {
                                conflicts.push(Conflict::PreconditionViolation {
                                    action: action_name.clone(),
                                    missing_condition: format!("{}={:?}", key, value),
                                });
                            }
                        } else {
                            conflicts.push(Conflict::PreconditionViolation {
                                action: action_name.clone(),
                                missing_condition: format!("{} (missing)", key),
                            });
                        }
                    }
                }

                // Check for state conflicts with future actions
                for (_j, future_action_name) in plan.iter().enumerate().skip(i + 1) {
                    if let Some(future_action) =
                        actions.iter().find(|a| a.name() == future_action_name)
                    {
                        // Check if effects conflict
                        for (key, value1) in action.effects() {
                            for (key2, value2) in future_action.effects() {
                                if key == key2 && value1 != value2 {
                                    // Potential conflict if both try to set same variable
                                    conflicts.push(Conflict::StateConflict {
                                        action1: action_name.clone(),
                                        action2: future_action_name.clone(),
                                        variable: key.clone(),
                                    });
                                }
                            }
                        }

                        // Check for known incompatible action pairs
                        if Self::are_incompatible(action_name, future_action_name) {
                            conflicts.push(Conflict::IncompatibleActions {
                                action1: action_name.clone(),
                                action2: future_action_name.clone(),
                                reason: "Actions cannot be performed simultaneously".to_string(),
                            });
                        }
                    }
                }

                // Apply effects for next iteration
                simulated_state.apply_effects(action.effects());
            }
        }

        conflicts
    }

    /// Check if two actions are known to be incompatible
    fn are_incompatible(action1: &str, action2: &str) -> bool {
        // Define known incompatible pairs
        // (In a real system, this might be data-driven)
        let incompatible_pairs = vec![
            ("attack", "heal"),       // Can't attack and heal at same time
            ("move_to", "move_to"),   // Conflicting movement
            ("take_cover", "attack"), // Can't attack while taking cover
        ];

        incompatible_pairs.iter().any(|(a, b)| {
            (action1.contains(a) && action2.contains(b))
                || (action1.contains(b) && action2.contains(a))
        })
    }

    /// Optimize a plan by removing redundant actions
    pub fn optimize(
        plan: Vec<String>,
        actions: &[Box<dyn Action>],
        start_state: &WorldState,
    ) -> Vec<String> {
        if plan.is_empty() {
            return plan;
        }

        let mut optimized = Vec::new();
        let mut simulated_state = start_state.clone();
        let mut applied_effects = HashSet::new();

        for action_name in plan {
            if let Some(action) = actions.iter().find(|a| a.name() == &action_name) {
                // Check if this action's effects are redundant
                let mut is_redundant = true;

                for (key, value) in action.effects() {
                    let effect_id = format!("{}={:?}", key, value);

                    // If we haven't seen this effect yet, it's not redundant
                    if !applied_effects.contains(&effect_id) {
                        is_redundant = false;
                        applied_effects.insert(effect_id);
                    }
                }

                // Keep non-redundant actions or actions with side effects
                if !is_redundant || action.effects().is_empty() {
                    optimized.push(action_name.clone());
                    simulated_state.apply_effects(action.effects());
                }
            } else {
                // Keep unknown actions (they might be important)
                optimized.push(action_name);
            }
        }

        optimized
    }

    /// Validate that a plan is executable from start to finish
    pub fn validate_plan(
        plan: &[String],
        actions: &[Box<dyn Action>],
        start_state: &WorldState,
    ) -> Result<(), StitchError> {
        let conflicts = Self::detect_conflicts(plan, actions, start_state);

        if !conflicts.is_empty() {
            return Err(StitchError::ConflictDetected(conflicts));
        }

        Ok(())
    }

    /// Find safe resume points in a plan (states where re-planning could occur)
    pub fn find_resume_points(
        plan: &[String],
        actions: &[Box<dyn Action>],
        start_state: &WorldState,
    ) -> Vec<usize> {
        let mut resume_points = vec![0]; // Start is always a resume point
        let mut simulated_state = start_state.clone();

        for (i, action_name) in plan.iter().enumerate() {
            if let Some(action) = actions.iter().find(|a| a.name() == action_name) {
                simulated_state.apply_effects(action.effects());

                // A resume point is where state is "stable"
                // For now, mark every N actions as potential resume point
                if (i + 1) % 3 == 0 {
                    resume_points.push(i + 1);
                }
            }
        }

        resume_points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goap::{SimpleAction, StateValue};
    use std::collections::BTreeMap;

    fn create_test_action(name: &str, effects: Vec<(&str, StateValue)>) -> Box<dyn Action> {
        let mut effects_map = BTreeMap::new();
        for (key, value) in effects {
            effects_map.insert(key.to_string(), value);
        }
        Box::new(SimpleAction::new(name, BTreeMap::new(), effects_map, 1.0))
    }

    #[test]
    fn test_merge_sequential() {
        let plan1 = vec!["action1".to_string(), "action2".to_string()];
        let plan2 = vec!["action3".to_string()];
        let plans = vec![plan1, plan2];

        let result = PlanStitcher::merge_sequential(plans).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "action1");
        assert_eq!(result[1], "action2");
        assert_eq!(result[2], "action3");
    }

    #[test]
    fn test_merge_sequential_empty() {
        let result = PlanStitcher::merge_sequential(vec![]);
        assert!(matches!(result, Err(StitchError::EmptyPlans)));
    }

    #[test]
    fn test_merge_interleaved() {
        let plan1 = vec!["a1".to_string(), "a2".to_string()];
        let plan2 = vec!["b1".to_string(), "b2".to_string()];
        let plans = vec![plan1, plan2];
        let priorities = vec![10.0, 5.0]; // plan1 has higher priority

        let result = PlanStitcher::merge_interleaved(plans, priorities).unwrap();
        assert_eq!(result.len(), 4);
        // Higher priority plan's actions should come first more often
        assert_eq!(result[0], "a1"); // High priority first
    }

    #[test]
    fn test_detect_conflicts_precondition() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("action1", vec![("x", StateValue::Int(1))]),
            create_test_action("action2", vec![("y", StateValue::Int(2))]),
        ];

        let plan = vec!["action1".to_string(), "action2".to_string()];
        let start_state = WorldState::new();

        let conflicts = PlanStitcher::detect_conflicts(&plan, &actions, &start_state);
        // Should execute without precondition conflicts
        assert_eq!(conflicts.len(), 0);
    }

    #[test]
    fn test_detect_conflicts_state() {
        let mut effects1 = BTreeMap::new();
        effects1.insert("health".to_string(), StateValue::Int(100));
        let action1 = Box::new(SimpleAction::new("heal", BTreeMap::new(), effects1, 1.0));

        let mut effects2 = BTreeMap::new();
        effects2.insert("health".to_string(), StateValue::Int(50));
        let action2 = Box::new(SimpleAction::new("damage", BTreeMap::new(), effects2, 1.0));

        let actions: Vec<Box<dyn Action>> = vec![action1, action2];
        let plan = vec!["heal".to_string(), "damage".to_string()];
        let start_state = WorldState::new();

        let conflicts = PlanStitcher::detect_conflicts(&plan, &actions, &start_state);
        // Should detect state conflict on "health"
        assert!(!conflicts.is_empty());
        assert!(matches!(conflicts[0], Conflict::StateConflict { .. }));
    }

    #[test]
    fn test_optimize_removes_redundant() {
        let mut effects = BTreeMap::new();
        effects.insert("flag".to_string(), StateValue::Bool(true));
        let action = Box::new(SimpleAction::new(
            "set_flag",
            BTreeMap::new(),
            effects.clone(),
            1.0,
        ));

        let actions: Vec<Box<dyn Action>> = vec![action];
        let plan = vec!["set_flag".to_string(), "set_flag".to_string()]; // Redundant
        let start_state = WorldState::new();

        let optimized = PlanStitcher::optimize(plan, &actions, &start_state);
        assert_eq!(optimized.len(), 1); // Second set_flag removed
    }

    #[test]
    fn test_validate_plan_success() {
        let actions: Vec<Box<dyn Action>> = vec![create_test_action(
            "action1",
            vec![("x", StateValue::Int(1))],
        )];

        let plan = vec!["action1".to_string()];
        let start_state = WorldState::new();

        let result = PlanStitcher::validate_plan(&plan, &actions, &start_state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_resume_points() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("a1", vec![("x", StateValue::Int(1))]),
            create_test_action("a2", vec![("y", StateValue::Int(2))]),
            create_test_action("a3", vec![("z", StateValue::Int(3))]),
            create_test_action("a4", vec![("w", StateValue::Int(4))]),
        ];

        let plan = vec![
            "a1".to_string(),
            "a2".to_string(),
            "a3".to_string(),
            "a4".to_string(),
        ];
        let start_state = WorldState::new();

        let resume_points = PlanStitcher::find_resume_points(&plan, &actions, &start_state);
        assert!(resume_points.contains(&0)); // Start
        assert!(resume_points.len() > 1); // Should have intermediate points
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    fn create_action_with_preconditions(
        name: &str,
        preconditions: Vec<(&str, StateValue)>,
        effects: Vec<(&str, StateValue)>,
    ) -> Box<dyn Action> {
        let mut prec_map = BTreeMap::new();
        for (k, v) in preconditions {
            prec_map.insert(k.to_string(), v);
        }
        let mut eff_map = BTreeMap::new();
        for (k, v) in effects {
            eff_map.insert(k.to_string(), v);
        }
        Box::new(SimpleAction::new(name, prec_map, eff_map, 1.0))
    }

    /// Kills lines 115, 119: detect_conflicts with unsatisfied preconditions
    #[test]
    fn test_detect_conflicts_unsatisfied_preconditions() {
        let actions: Vec<Box<dyn Action>> = vec![create_action_with_preconditions(
            "needs_ammo",
            vec![("has_ammo", StateValue::Bool(true))],
            vec![("fired", StateValue::Bool(true))],
        )];
        let plan = vec!["needs_ammo".to_string()];
        // Start state lacks has_ammo, so precondition is violated
        let start_state = WorldState::new();

        let conflicts = PlanStitcher::detect_conflicts(&plan, &actions, &start_state);
        assert!(
            !conflicts.is_empty(),
            "Should detect precondition violation"
        );
        assert!(
            matches!(conflicts[0], Conflict::PreconditionViolation { .. }),
            "Should be PreconditionViolation"
        );
    }

    /// Kills lines 115, 119: detect_conflicts with precondition value mismatch
    #[test]
    fn test_detect_conflicts_precondition_value_mismatch() {
        let actions: Vec<Box<dyn Action>> = vec![create_action_with_preconditions(
            "needs_high_health",
            vec![("health", StateValue::Int(100))],
            vec![("attacked", StateValue::Bool(true))],
        )];
        let plan = vec!["needs_high_health".to_string()];
        let mut start_state = WorldState::new();
        start_state.set("health", StateValue::Int(50)); // Wrong value

        let conflicts = PlanStitcher::detect_conflicts(&plan, &actions, &start_state);
        assert!(!conflicts.is_empty(), "Should detect value mismatch");
    }

    /// Kills line 135: skip(i + 1) offset with 3+ actions
    #[test]
    fn test_detect_conflicts_state_conflict_three_actions() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("a1", vec![("health", StateValue::Int(100))]),
            create_test_action("a2", vec![("ammo", StateValue::Int(10))]),
            create_test_action("a3", vec![("health", StateValue::Int(50))]),
        ];
        let plan = vec!["a1".to_string(), "a2".to_string(), "a3".to_string()];
        let start_state = WorldState::new();

        let conflicts = PlanStitcher::detect_conflicts(&plan, &actions, &start_state);
        // a1 sets health=100, a3 sets health=50 → state conflict
        let state_conflicts: Vec<_> = conflicts
            .iter()
            .filter(|c| matches!(c, Conflict::StateConflict { .. }))
            .collect();
        assert!(
            !state_conflicts.is_empty(),
            "Should detect state conflict between non-adjacent actions"
        );
    }

    /// Kills line 176: "attack" + "heal" incompatibility
    /// Kills lines 183-184: reverse order check
    #[test]
    fn test_are_incompatible_attack_heal() {
        assert!(
            PlanStitcher::are_incompatible("attack_enemy", "heal_ally"),
            "attack + heal should be incompatible"
        );
        assert!(
            PlanStitcher::are_incompatible("heal_ally", "attack_enemy"),
            "heal + attack should be incompatible (reverse)"
        );
        assert!(
            !PlanStitcher::are_incompatible("move_forward", "look_around"),
            "unrelated actions should not be incompatible"
        );
    }

    /// Kills line 194: optimize(vec![]) returning non-empty
    #[test]
    fn test_optimize_empty_plan() {
        let actions: Vec<Box<dyn Action>> = vec![];
        let start_state = WorldState::new();
        let result = PlanStitcher::optimize(vec![], &actions, &start_state);
        assert!(
            result.is_empty(),
            "Optimizing empty plan should return empty"
        );
    }

    /// Kills line 218: unknown action preserved in optimize
    #[test]
    fn test_optimize_preserves_unknown_actions() {
        let actions: Vec<Box<dyn Action>> = vec![create_test_action(
            "known",
            vec![("x", StateValue::Bool(true))],
        )];
        let plan = vec!["known".to_string(), "unknown_action".to_string()];
        let start_state = WorldState::new();

        let result = PlanStitcher::optimize(plan, &actions, &start_state);
        assert!(
            result.contains(&"unknown_action".to_string()),
            "Unknown actions should be preserved"
        );
    }

    /// Kills line 237: validate_plan with conflicts returns Err
    #[test]
    fn test_validate_plan_with_conflicts() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("attack_enemy", vec![("damage", StateValue::Int(10))]),
            create_test_action("heal_target", vec![("healed", StateValue::Bool(true))]),
        ];
        let plan = vec!["attack_enemy".to_string(), "heal_target".to_string()];
        let start_state = WorldState::new();

        let result = PlanStitcher::validate_plan(&plan, &actions, &start_state);
        assert!(
            result.is_err(),
            "validate_plan should fail for incompatible actions"
        );
    }

    /// Kills lines 256, 261, 262: find_resume_points exact indices
    #[test]
    fn test_find_resume_points_exact_indices() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("a1", vec![("v1", StateValue::Int(1))]),
            create_test_action("a2", vec![("v2", StateValue::Int(2))]),
            create_test_action("a3", vec![("v3", StateValue::Int(3))]),
            create_test_action("a4", vec![("v4", StateValue::Int(4))]),
            create_test_action("a5", vec![("v5", StateValue::Int(5))]),
            create_test_action("a6", vec![("v6", StateValue::Int(6))]),
        ];
        let plan = vec![
            "a1".to_string(),
            "a2".to_string(),
            "a3".to_string(),
            "a4".to_string(),
            "a5".to_string(),
            "a6".to_string(),
        ];
        let start_state = WorldState::new();

        let resume_points = PlanStitcher::find_resume_points(&plan, &actions, &start_state);
        // Start is always 0, then every 3rd: (0+1)%3!=0, (1+1)%3!=0, (2+1)%3==0→push 3,
        // (3+1)%3!=0, (4+1)%3!=0, (5+1)%3==0→push 6
        assert_eq!(
            resume_points,
            vec![0, 3, 6],
            "Resume points should be exactly [0, 3, 6] for 6 actions"
        );
    }

    /// Kills lines 183-184: `&&` → `||` in are_incompatible
    /// Tests that "attack" + "look" are NOT incompatible (only one token matches).
    #[test]
    fn test_are_incompatible_rejects_false_positives() {
        // "attack" matches in pair ("attack","heal") but "look" doesn't match "heal"
        // With && → ||, "attack" alone would trigger incompatibility
        assert!(
            !PlanStitcher::are_incompatible("attack_enemy", "look_around"),
            "attack + look_around should NOT be incompatible"
        );
        assert!(
            !PlanStitcher::are_incompatible("look_around", "attack_enemy"),
            "look_around + attack_enemy should NOT be incompatible (reverse)"
        );
    }

    /// Kills line 218: `delete !` in optimize (is_redundant → !is_redundant)
    /// Verifies non-redundant actions with effects are kept.
    #[test]
    fn test_optimize_keeps_non_redundant_actions() {
        let actions: Vec<Box<dyn Action>> = vec![
            create_test_action("a1", vec![("x", StateValue::Int(1))]),
            create_test_action("a2", vec![("y", StateValue::Int(2))]),
        ];
        let plan = vec!["a1".to_string(), "a2".to_string()];
        let start_state = WorldState::new();

        let result = PlanStitcher::optimize(plan.clone(), &actions, &start_state);
        assert_eq!(
            result, plan,
            "Non-redundant actions should all be preserved"
        );
    }
}
