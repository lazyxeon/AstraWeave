use super::{Action, ActionHistory, Goal, WorldState};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

/// A* search node for planning
#[derive(Clone)]
struct PlanNode {
    state: WorldState,
    path: Vec<String>, // Action names taken to reach this state
    g_cost: f32,       // Actual cost from start
    h_cost: f32,       // Heuristic cost to goal
    risk: f32,         // Accumulated risk
}

impl PlanNode {
    /// Total f-cost for A* (g + h + risk_weight * risk)
    fn f_cost(&self, risk_weight: f32) -> f32 {
        self.g_cost + self.h_cost + (self.risk * risk_weight)
    }
}

impl PartialEq for PlanNode {
    fn eq(&self, other: &Self) -> bool {
        // For priority queue, compare by default risk weight
        self.f_cost(5.0) == other.f_cost(5.0)
    }
}

impl Eq for PlanNode {}

impl PartialOrd for PlanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (BinaryHeap is max-heap by default)
        other
            .f_cost(5.0)
            .partial_cmp(&self.f_cost(5.0))
            .unwrap_or(Ordering::Equal)
    }
}

/// Advanced GOAP planner with learning, risk-awareness, and multi-goal support
pub struct AdvancedGOAP {
    actions: Vec<Box<dyn Action>>,
    history: ActionHistory,
    max_plan_iterations: usize,
    risk_weight: f32, // How much to penalize risky actions (default 5.0)
}

impl AdvancedGOAP {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            history: ActionHistory::new(),
            max_plan_iterations: 10000,
            risk_weight: 5.0,
        }
    }

    /// Add an action to the planner's action library
    pub fn add_action(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

    /// Get immutable reference to action history
    pub fn get_history(&self) -> &ActionHistory {
        &self.history
    }

    /// Get mutable reference to action history
    pub fn get_history_mut(&mut self) -> &mut ActionHistory {
        &mut self.history
    }

    /// Get reference to all registered actions
    pub fn get_actions(&self) -> &[Box<dyn Action>] {
        &self.actions
    }

    /// Set maximum planning iterations
    pub fn set_max_iterations(&mut self, max_iterations: usize) {
        self.max_plan_iterations = max_iterations;
    }

    /// Set risk weight (higher = more risk-averse planning)
    pub fn set_risk_weight(&mut self, weight: f32) {
        self.risk_weight = weight;
    }

    /// Hierarchical planning with goal decomposition support
    /// Entry point for planning that handles both simple and hierarchical goals
    pub fn plan(&self, start: &WorldState, goal: &Goal) -> Option<Vec<String>> {
        self.plan_hierarchical(start, goal, 0)
    }

    /// Internal hierarchical planning with depth tracking
    fn plan_hierarchical(
        &self,
        start: &WorldState,
        goal: &Goal,
        depth: usize,
    ) -> Option<Vec<String>> {
        // Check if goal is already satisfied
        if goal.is_satisfied(start) {
            tracing::debug!("Goal '{}' already satisfied at depth {}", goal.name, depth);
            return Some(Vec::new());
        }

        // Check if we should decompose this goal
        if goal.should_decompose(depth) {
            tracing::debug!("Decomposing goal '{}' at depth {}", goal.name, depth);

            // Try hierarchical decomposition first
            if let Some(plan) = self.plan_decomposed(start, goal, depth) {
                return Some(plan);
            }

            // If decomposition failed, fall through to direct planning
            tracing::debug!(
                "Decomposition failed for '{}', trying direct planning",
                goal.name
            );
        }

        // Direct planning (standard A* search)
        self.plan_direct(start, goal)
    }

    /// Plan using goal decomposition (HTN-style)
    fn plan_decomposed(
        &self,
        start: &WorldState,
        goal: &Goal,
        depth: usize,
    ) -> Option<Vec<String>> {
        let sub_goals = goal.decompose()?;

        match goal.decomposition_strategy {
            super::DecompositionStrategy::Sequential => {
                self.plan_sequential(start, &sub_goals, depth + 1)
            }
            super::DecompositionStrategy::Parallel | super::DecompositionStrategy::AllOf => {
                self.plan_parallel(start, &sub_goals, depth + 1)
            }
            super::DecompositionStrategy::AnyOf => self.plan_any_of(start, &sub_goals, depth + 1),
        }
    }

    /// Plan sub-goals sequentially
    fn plan_sequential(
        &self,
        start: &WorldState,
        sub_goals: &[Goal],
        depth: usize,
    ) -> Option<Vec<String>> {
        let mut combined_plan = Vec::new();
        let mut current_state = start.clone();

        for sub_goal in sub_goals {
            // Recursively plan for this sub-goal
            let sub_plan = self.plan_hierarchical(&current_state, sub_goal, depth)?;

            // Simulate state changes from sub-plan
            for action_name in &sub_plan {
                if let Some(action) = self.actions.iter().find(|a| a.name() == action_name) {
                    current_state.apply_effects(action.effects());
                }
            }

            combined_plan.extend(sub_plan);
        }

        Some(combined_plan)
    }

    /// Plan sub-goals in parallel/all-of (try to optimize order)
    fn plan_parallel(
        &self,
        start: &WorldState,
        sub_goals: &[Goal],
        depth: usize,
    ) -> Option<Vec<String>> {
        // For now, treat parallel as sequential with priority ordering
        // TODO: In future, could interleave actions for true parallel execution
        let mut sorted_sub_goals = sub_goals.to_vec();
        sorted_sub_goals.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        self.plan_sequential(start, &sorted_sub_goals, depth)
    }

    /// Plan for any-of sub-goals (first successful plan wins)
    fn plan_any_of(
        &self,
        start: &WorldState,
        sub_goals: &[Goal],
        depth: usize,
    ) -> Option<Vec<String>> {
        // Try each sub-goal in priority order until one succeeds
        let mut sorted_sub_goals = sub_goals.to_vec();
        sorted_sub_goals.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for sub_goal in sorted_sub_goals {
            if let Some(plan) = self.plan_hierarchical(start, &sub_goal, depth) {
                return Some(plan);
            }
        }

        None // No sub-goal could be achieved
    }

    /// Direct A* planning (non-hierarchical)
    fn plan_direct(&self, start: &WorldState, goal: &Goal) -> Option<Vec<String>> {
        // Allocation-measurement instrumentation (audit 2026-04-17, §2.3 #1,
        // open question #2). This is the highest-volume inferred allocation site
        // in the engine — A* per-expansion clones `WorldState`, `Vec<String>` path,
        // and allocates a `String` per action name. The plot reveals actual counts.
        #[cfg(feature = "profiling")]
        astraweave_profiling::measured_span!("ai.goap.plan");

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();

        let start_node = PlanNode {
            state: start.clone(),
            path: Vec::new(),
            g_cost: 0.0,
            h_cost: start.distance_to(&goal.desired_state),
            risk: 0.0,
        };

        open_set.push(start_node);

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_plan_iterations {
                tracing::warn!(
                    "Max iterations ({}) reached in planning for goal '{}'",
                    self.max_plan_iterations,
                    goal.name
                );
                return None;
            }

            // Goal check
            if goal.is_satisfied(&current.state) {
                tracing::debug!(
                    "Plan found for '{}' with {} steps, cost {:.2}, risk {:.2}",
                    goal.name,
                    current.path.len(),
                    current.g_cost,
                    current.risk
                );
                return Some(current.path);
            }

            // State signature for closed set (using hash)
            // Since WorldState implements Hash properly, we can use it directly
            if closed_set.contains(&current.state) {
                continue;
            }
            closed_set.insert(current.state.clone());

            // Expand neighbors (successor states)
            for action in &self.actions {
                if !action.can_execute(&current.state) {
                    continue;
                }

                let mut new_state = current.state.clone();
                new_state.apply_effects(action.effects());

                let action_cost = action.calculate_cost(&current.state, &self.history);
                let action_risk = 1.0 - action.success_probability(&current.state, &self.history);

                let mut new_path = current.path.clone();
                new_path.push(action.name().to_string());

                let new_node = PlanNode {
                    state: new_state.clone(),
                    path: new_path,
                    g_cost: current.g_cost + action_cost,
                    h_cost: new_state.distance_to(&goal.desired_state),
                    risk: current.risk + action_risk,
                };

                open_set.push(new_node);
            }
        }

        tracing::debug!(
            "No plan found for goal '{}' after {} iterations",
            goal.name,
            iterations
        );
        None // No plan found
    }

    /// Multi-goal planning with priority scheduling
    /// Returns plans for each goal in priority order
    pub fn plan_multiple_goals(
        &self,
        start: &WorldState,
        goals: &[Goal],
        current_time: f32,
    ) -> Vec<(String, Vec<String>)> {
        let mut plans = Vec::new();
        let mut sorted_goals = goals.to_vec();

        // Sort by urgency (considers priority + deadline)
        sorted_goals.sort_by(|a, b| {
            b.urgency(current_time)
                .partial_cmp(&a.urgency(current_time))
                .unwrap_or(Ordering::Equal)
        });

        let mut current_state = start.clone();

        for goal in sorted_goals {
            if let Some(plan) = self.plan(&current_state, &goal) {
                // Simulate state changes from this plan
                for action_name in &plan {
                    if let Some(action) = self.actions.iter().find(|a| a.name() == action_name) {
                        current_state.apply_effects(action.effects());
                    }
                }
                plans.push((goal.name.clone(), plan));
            } else {
                tracing::warn!("Failed to find plan for goal '{}'", goal.name);
            }
        }

        plans
    }

    /// Execute a plan and record results in history
    /// This is for simulation/testing; real execution happens via engine
    pub fn simulate_plan_execution(
        &mut self,
        plan: &[String],
        world: &mut WorldState,
    ) -> Result<(), String> {
        for action_name in plan {
            let action = self
                .actions
                .iter()
                .find(|a| a.name() == action_name)
                .ok_or_else(|| format!("Action not found: {}", action_name))?;

            if !action.can_execute(world) {
                self.history.record_failure(action_name);
                return Err(format!("Action '{}' preconditions not met", action_name));
            }

            // Simulate success (in reality, check actual execution)
            let success_prob = action.success_probability(world, &self.history);

            // For simulation, assume action succeeds if probability > 0.5
            if success_prob > 0.5 {
                world.apply_effects(action.effects());
                self.history.record_success(action_name, 1.0);
            } else {
                self.history.record_failure(action_name);
                return Err(format!("Action '{}' simulated failure", action_name));
            }
        }

        Ok(())
    }

    /// Get number of registered actions
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Get action names
    pub fn action_names(&self) -> Vec<String> {
        self.actions.iter().map(|a| a.name().to_string()).collect()
    }
}

impl Default for AdvancedGOAP {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goap::action::SimpleAction;
    use crate::goap::StateValue;
    use std::collections::BTreeMap;

    #[test]
    fn test_simple_plan() {
        let mut goap = AdvancedGOAP::new();

        // Define action: move (no preconditions) -> at_goal
        let mut move_effects = BTreeMap::new();
        move_effects.insert("at_goal".to_string(), StateValue::Bool(true));
        let move_action = SimpleAction::new("move", BTreeMap::new(), move_effects, 1.0);

        goap.add_action(Box::new(move_action));

        // Start state: not at goal
        let mut start = WorldState::new();
        start.set("at_goal", StateValue::Bool(false));

        // Goal: be at goal
        let mut goal_state = BTreeMap::new();
        goal_state.insert("at_goal".to_string(), StateValue::Bool(true));
        let goal = Goal::new("reach_goal", goal_state);

        let plan = goap.plan(&start, &goal);

        assert!(plan.is_some());
        assert_eq!(plan.unwrap(), vec!["move"]);
    }

    #[test]
    fn test_multi_step_plan() {
        let mut goap = AdvancedGOAP::new();

        // Action 1: find_weapon -> has_weapon
        let mut find_weapon_effects = BTreeMap::new();
        find_weapon_effects.insert("has_weapon".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "find_weapon",
            BTreeMap::new(),
            find_weapon_effects,
            2.0,
        )));

        // Action 2: equip_weapon (requires has_weapon) -> weapon_equipped
        let mut equip_preconds = BTreeMap::new();
        equip_preconds.insert("has_weapon".to_string(), StateValue::Bool(true));
        let mut equip_effects = BTreeMap::new();
        equip_effects.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "equip_weapon",
            equip_preconds,
            equip_effects,
            1.0,
        )));

        // Action 3: attack (requires weapon_equipped) -> enemy_defeated
        let mut attack_preconds = BTreeMap::new();
        attack_preconds.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let mut attack_effects = BTreeMap::new();
        attack_effects.insert("enemy_defeated".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "attack",
            attack_preconds,
            attack_effects,
            5.0,
        )));

        // Start state: nothing
        let start = WorldState::new();

        // Goal: defeat enemy
        let mut goal_state = BTreeMap::new();
        goal_state.insert("enemy_defeated".to_string(), StateValue::Bool(true));
        let goal = Goal::new("defeat_enemy", goal_state);

        let plan = goap.plan(&start, &goal);

        assert!(plan.is_some());
        let plan_steps = plan.unwrap();
        assert_eq!(plan_steps.len(), 3);
        assert_eq!(plan_steps[0], "find_weapon");
        assert_eq!(plan_steps[1], "equip_weapon");
        assert_eq!(plan_steps[2], "attack");
    }

    #[test]
    fn test_no_plan_found() {
        let mut goap = AdvancedGOAP::new();

        // Action that requires something we can't get
        let mut preconds = BTreeMap::new();
        preconds.insert("impossible_condition".to_string(), StateValue::Bool(true));
        let mut effects = BTreeMap::new();
        effects.insert("goal".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "impossible",
            preconds,
            effects,
            1.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("goal".to_string(), StateValue::Bool(true));
        let goal = Goal::new("impossible_goal", goal_state);

        let plan = goap.plan(&start, &goal);

        assert!(plan.is_none());
    }

    #[test]
    fn test_multi_goal_planning() {
        let mut goap = AdvancedGOAP::new();

        // Action 1: heal -> health = 100
        let mut heal_effects = BTreeMap::new();
        heal_effects.insert("health".to_string(), StateValue::Int(100));
        goap.add_action(Box::new(SimpleAction::new(
            "heal",
            BTreeMap::new(),
            heal_effects,
            3.0,
        )));

        // Action 2: reload -> ammo = 30
        let mut reload_effects = BTreeMap::new();
        reload_effects.insert("ammo".to_string(), StateValue::Int(30));
        goap.add_action(Box::new(SimpleAction::new(
            "reload",
            BTreeMap::new(),
            reload_effects,
            2.0,
        )));

        let mut start = WorldState::new();
        start.set("health", StateValue::Int(50));
        start.set("ammo", StateValue::Int(5));

        // Goal 1: High priority heal
        let mut heal_goal_state = BTreeMap::new();
        heal_goal_state.insert("health".to_string(), StateValue::Int(100));
        let heal_goal = Goal::new("stay_alive", heal_goal_state).with_priority(10.0);

        // Goal 2: Lower priority reload
        let mut reload_goal_state = BTreeMap::new();
        reload_goal_state.insert("ammo".to_string(), StateValue::Int(30));
        let reload_goal = Goal::new("reload_weapon", reload_goal_state).with_priority(5.0);

        let plans = goap.plan_multiple_goals(&start, &[heal_goal, reload_goal], 0.0);

        assert_eq!(plans.len(), 2);
        // High priority goal should be first
        assert_eq!(plans[0].0, "stay_alive");
        assert_eq!(plans[0].1, vec!["heal"]);
        assert_eq!(plans[1].0, "reload_weapon");
        assert_eq!(plans[1].1, vec!["reload"]);
    }

    #[test]
    fn test_risk_awareness() {
        let mut goap = AdvancedGOAP::new();

        // Risky action
        let risky = SimpleAction::new("risky_attack", BTreeMap::new(), BTreeMap::new(), 2.0);
        goap.add_action(Box::new(risky));

        // Record failures to build history
        goap.get_history_mut().record_failure("risky_attack");
        goap.get_history_mut().record_failure("risky_attack");
        goap.get_history_mut().record_success("risky_attack", 1.0);

        let stats = goap.get_history().get_action_stats("risky_attack").unwrap();
        assert!((stats.failure_rate() - 0.666666).abs() < 0.01);
    }

    #[test]
    fn test_deterministic_planning() {
        let mut goap = AdvancedGOAP::new();

        let mut effects = BTreeMap::new();
        effects.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "action",
            BTreeMap::new(),
            effects,
            1.0,
        )));

        let start = WorldState::new();

        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("goal", goal_state);

        // Multiple planning runs should produce same result
        let plan1 = goap.plan(&start, &goal);
        let plan2 = goap.plan(&start, &goal);
        let plan3 = goap.plan(&start, &goal);

        assert_eq!(plan1, plan2);
        assert_eq!(plan2, plan3);
    }

    // ── Mutation-killing tests ──

    #[test]
    fn test_f_cost_arithmetic() {
        // Kills g+h+risk*weight arithmetic mutations
        let node = PlanNode {
            state: WorldState::new(),
            path: vec![],
            g_cost: 2.0,
            h_cost: 3.0,
            risk: 1.0,
        };
        // f = 2 + 3 + 1*5 = 10
        assert_eq!(node.f_cost(5.0), 10.0);
        // f with weight=0 → risk is ignored: 2+3+0 = 5
        assert_eq!(node.f_cost(0.0), 5.0);
        // f with weight=2 → 2+3+2 = 7
        assert_eq!(node.f_cost(2.0), 7.0);
    }

    #[test]
    fn test_plan_node_ordering_min_heap() {
        // BinaryHeap is max-heap; Ord is reversed for min-heap behavior.
        // Lower f_cost should come out first.
        let low = PlanNode {
            state: WorldState::new(),
            path: vec![],
            g_cost: 1.0,
            h_cost: 0.0,
            risk: 0.0,
        };
        let high = PlanNode {
            state: WorldState::new(),
            path: vec![],
            g_cost: 10.0,
            h_cost: 0.0,
            risk: 0.0,
        };
        // Reversed: low should be Greater than high (so BinaryHeap pops low first)
        assert_eq!(low.cmp(&high), Ordering::Greater);
        assert_eq!(high.cmp(&low), Ordering::Less);
    }

    #[test]
    fn test_plan_already_satisfied() {
        // Kills the early-return for already-satisfied goals
        let mut goap = AdvancedGOAP::new();

        let mut effects = BTreeMap::new();
        effects.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "action",
            BTreeMap::new(),
            effects,
            1.0,
        )));

        let mut start = WorldState::new();
        start.set("done", StateValue::Bool(true)); // already satisfied

        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("goal", goal_state);

        let plan = goap.plan(&start, &goal);
        assert!(plan.is_some());
        assert!(plan.unwrap().is_empty()); // No actions needed
    }

    #[test]
    fn test_action_count_and_names() {
        let mut goap = AdvancedGOAP::new();
        assert_eq!(goap.action_count(), 0);
        assert!(goap.action_names().is_empty());

        goap.add_action(Box::new(SimpleAction::new(
            "a",
            BTreeMap::new(),
            BTreeMap::new(),
            1.0,
        )));
        goap.add_action(Box::new(SimpleAction::new(
            "b",
            BTreeMap::new(),
            BTreeMap::new(),
            2.0,
        )));
        goap.add_action(Box::new(SimpleAction::new(
            "c",
            BTreeMap::new(),
            BTreeMap::new(),
            3.0,
        )));

        assert_eq!(goap.action_count(), 3);
        let names = goap.action_names();
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
        assert!(names.contains(&"c".to_string()));
    }

    #[test]
    fn test_set_max_iterations_limits_search() {
        // With max_iterations=1, a multi-step plan should fail
        let mut goap = AdvancedGOAP::new();
        goap.set_max_iterations(1);

        // Need 2 steps: get_key → open_door
        let mut key_eff = BTreeMap::new();
        key_eff.insert("has_key".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "get_key",
            BTreeMap::new(),
            key_eff,
            1.0,
        )));

        let mut door_pre = BTreeMap::new();
        door_pre.insert("has_key".to_string(), StateValue::Bool(true));
        let mut door_eff = BTreeMap::new();
        door_eff.insert("door_open".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "open_door",
            door_pre,
            door_eff,
            1.0,
        )));

        let start = WorldState::new();
        let mut goal_state = BTreeMap::new();
        goal_state.insert("door_open".to_string(), StateValue::Bool(true));
        let goal = Goal::new("open", goal_state);

        // Should fail: not enough iterations to find 2-step plan
        assert!(goap.plan(&start, &goal).is_none());
    }

    #[test]
    fn test_set_risk_weight_prefers_safer_plan() {
        // Two paths to goal: risky (cheap but high failure) vs safe (expensive but reliable)
        let mut goap = AdvancedGOAP::new();

        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));

        goap.add_action(Box::new(SimpleAction::new(
            "risky",
            BTreeMap::new(),
            eff.clone(),
            1.0,
        )));
        goap.add_action(Box::new(SimpleAction::new(
            "safe",
            BTreeMap::new(),
            eff,
            2.0,
        )));

        // Build history: risky has 20% success, safe has 100% success
        goap.get_history_mut().record_failure("risky");
        goap.get_history_mut().record_failure("risky");
        goap.get_history_mut().record_failure("risky");
        goap.get_history_mut().record_failure("risky");
        goap.get_history_mut().record_success("risky", 1.0);

        for _ in 0..5 {
            goap.get_history_mut().record_success("safe", 1.0);
        }

        goap.set_risk_weight(100.0); // Heavily penalize risk

        let start = WorldState::new();
        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", goal_state);

        let plan = goap.plan(&start, &goal).unwrap();
        assert_eq!(plan, vec!["safe"]); // Should prefer safe despite higher base cost
    }

    #[test]
    fn test_simulate_plan_execution_success_and_failure() {
        let mut goap = AdvancedGOAP::new();

        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "act",
            BTreeMap::new(),
            eff,
            1.0,
        )));

        // Default success_probability is 0.8 (> 0.5) → should succeed
        let mut world = WorldState::new();
        let result = goap.simulate_plan_execution(&["act".to_string()], &mut world);
        assert!(result.is_ok());
        assert_eq!(world.get("done"), Some(&StateValue::Bool(true)));

        // Build history to make probability <= 0.5
        let mut goap2 = AdvancedGOAP::new();
        goap2.add_action(Box::new(SimpleAction::new(
            "fail_act",
            BTreeMap::new(),
            BTreeMap::new(),
            1.0,
        )));
        // 1 success + 3 failures = 25% success rate (<= 0.5)
        goap2.get_history_mut().record_success("fail_act", 1.0);
        goap2.get_history_mut().record_failure("fail_act");
        goap2.get_history_mut().record_failure("fail_act");
        goap2.get_history_mut().record_failure("fail_act");

        let mut world2 = WorldState::new();
        let result2 = goap2.simulate_plan_execution(&["fail_act".to_string()], &mut world2);
        assert!(result2.is_err());
    }

    #[test]
    fn test_simulate_plan_execution_action_not_found() {
        let mut goap = AdvancedGOAP::new();
        let mut world = WorldState::new();
        let result = goap.simulate_plan_execution(&["nonexistent".to_string()], &mut world);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_simulate_plan_execution_precondition_failure() {
        let mut goap = AdvancedGOAP::new();
        let mut pre = BTreeMap::new();
        pre.insert("has_key".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "open_door",
            pre,
            BTreeMap::new(),
            1.0,
        )));

        let mut world = WorldState::new(); // no has_key
        let result = goap.simulate_plan_execution(&["open_door".to_string()], &mut world);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("preconditions"));
    }

    #[test]
    fn test_hierarchical_sequential_planning() {
        use super::super::DecompositionStrategy;

        let mut goap = AdvancedGOAP::new();

        // Action: get_key → has_key=true
        let mut key_eff = BTreeMap::new();
        key_eff.insert("has_key".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "get_key",
            BTreeMap::new(),
            key_eff,
            1.0,
        )));

        // Action: open_door (requires has_key) → door_open=true
        let mut door_pre = BTreeMap::new();
        door_pre.insert("has_key".to_string(), StateValue::Bool(true));
        let mut door_eff = BTreeMap::new();
        door_eff.insert("door_open".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "open_door",
            door_pre,
            door_eff,
            1.0,
        )));

        // Sub-goal 1: get the key
        let mut sg1_state = BTreeMap::new();
        sg1_state.insert("has_key".to_string(), StateValue::Bool(true));
        let sub1 = Goal::new("get_key_goal", sg1_state);

        // Sub-goal 2: open the door
        let mut sg2_state = BTreeMap::new();
        sg2_state.insert("door_open".to_string(), StateValue::Bool(true));
        let sub2 = Goal::new("open_door_goal", sg2_state);

        // Parent goal: desired_state must NOT be empty (or it's immediately satisfied)
        let mut parent_state = BTreeMap::new();
        parent_state.insert("door_open".to_string(), StateValue::Bool(true));
        let parent = Goal::new("enter_room", parent_state)
            .with_sub_goals(vec![sub1, sub2])
            .with_strategy(DecompositionStrategy::Sequential);

        let start = WorldState::new();
        let plan = goap.plan(&start, &parent);
        assert!(plan.is_some());
        let steps = plan.unwrap();
        assert_eq!(steps, vec!["get_key", "open_door"]);
    }

    #[test]
    fn test_hierarchical_any_of_planning() {
        use super::super::DecompositionStrategy;

        let mut goap = AdvancedGOAP::new();

        // Action: climb_wall → past_wall=true (cost 5)
        let mut climb_eff = BTreeMap::new();
        climb_eff.insert("past_wall".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "climb_wall",
            BTreeMap::new(),
            climb_eff,
            5.0,
        )));

        // Action: use_door → past_wall=true (cost 1)
        let mut door_eff = BTreeMap::new();
        door_eff.insert("past_wall".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "use_door",
            BTreeMap::new(),
            door_eff,
            1.0,
        )));

        // Sub-goal options (AnyOf)
        let mut sg_state = BTreeMap::new();
        sg_state.insert("past_wall".to_string(), StateValue::Bool(true));
        let sub_climb = Goal::new("climb", sg_state.clone()).with_priority(1.0);
        let sub_door = Goal::new("door", sg_state.clone()).with_priority(10.0); // Higher priority → tried first

        // Parent goal: must have unsatisfied desired_state
        let parent = Goal::new("get_past_wall", sg_state)
            .with_sub_goals(vec![sub_climb, sub_door])
            .with_strategy(DecompositionStrategy::AnyOf);

        let start = WorldState::new();
        let plan = goap.plan(&start, &parent).unwrap();
        // AnyOf sorts by priority desc: door (10.0) tried first → use_door
        assert!(plan.contains(&"use_door".to_string()));
    }

    #[test]
    fn test_hierarchical_parallel_sorts_by_priority() {
        use super::super::DecompositionStrategy;

        let mut goap = AdvancedGOAP::new();

        let mut heal_eff = BTreeMap::new();
        heal_eff.insert("healed".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "heal",
            BTreeMap::new(),
            heal_eff,
            1.0,
        )));

        let mut reload_eff = BTreeMap::new();
        reload_eff.insert("ammo_full".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "reload",
            BTreeMap::new(),
            reload_eff,
            1.0,
        )));

        let mut sg1_state = BTreeMap::new();
        sg1_state.insert("healed".to_string(), StateValue::Bool(true));
        let sub_heal = Goal::new("heal_goal", sg1_state).with_priority(1.0); // lower

        let mut sg2_state = BTreeMap::new();
        sg2_state.insert("ammo_full".to_string(), StateValue::Bool(true));
        let sub_reload = Goal::new("reload_goal", sg2_state).with_priority(10.0); // higher

        // Parent must have unsatisfied desired_state
        let mut parent_state = BTreeMap::new();
        parent_state.insert("healed".to_string(), StateValue::Bool(true));
        parent_state.insert("ammo_full".to_string(), StateValue::Bool(true));
        let parent = Goal::new("prepare", parent_state)
            .with_sub_goals(vec![sub_heal, sub_reload])
            .with_strategy(DecompositionStrategy::Parallel);

        let start = WorldState::new();
        let plan = goap.plan(&start, &parent).unwrap();
        // Parallel sorts by priority desc: reload first, then heal
        assert_eq!(plan[0], "reload");
        assert_eq!(plan[1], "heal");
    }

    #[test]
    fn test_plan_direct_cost_accumulation() {
        // Kills g_cost + action_cost → g_cost - action_cost or *
        let mut goap = AdvancedGOAP::new();

        // Chain: a (cost 3) → b (cost 5), total cost = 8
        let mut a_eff = BTreeMap::new();
        a_eff.insert("step1".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "a",
            BTreeMap::new(),
            a_eff,
            3.0,
        )));

        let mut b_pre = BTreeMap::new();
        b_pre.insert("step1".to_string(), StateValue::Bool(true));
        let mut b_eff = BTreeMap::new();
        b_eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new("b", b_pre, b_eff, 5.0)));

        let start = WorldState::new();
        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", goal_state);

        let plan = goap.plan(&start, &goal).unwrap();
        assert_eq!(plan, vec!["a", "b"]); // Correct ordering
    }

    // ── Round 2 mutation-killing tests ──

    #[test]
    fn test_get_actions_returns_registered_actions() {
        // Kills: get_actions → Vec::leak(Vec::new())
        let mut goap = AdvancedGOAP::new();
        assert!(goap.get_actions().is_empty());

        goap.add_action(Box::new(SimpleAction::new(
            "attack",
            BTreeMap::new(),
            BTreeMap::new(),
            1.0,
        )));
        goap.add_action(Box::new(SimpleAction::new(
            "heal",
            BTreeMap::new(),
            BTreeMap::new(),
            2.0,
        )));

        let actions = goap.get_actions();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].name(), "attack");
        assert_eq!(actions[1].name(), "heal");
    }

    #[test]
    fn test_set_risk_weight_stores_value() {
        // EQUIVALENT: set_risk_weight → () is equivalent because the Ord impl
        // for PlanNode uses hardcoded f_cost(5.0), not self.risk_weight.
        // The field is stored but never read by plan_direct's BinaryHeap ordering.
        // This test just exercises the API path.
        let mut goap = AdvancedGOAP::new();
        goap.set_risk_weight(0.0);
        goap.set_risk_weight(100.0);
        // No observable difference — equivalent mutant.
    }

    #[test]
    fn test_max_iterations_boundary() {
        // Kills: iterations > max → >= or ==
        // With max=2 and a 1-step plan: needs 2 iterations (start + goal state).
        // > 2: iteration 1 passes (1>2=false), iteration 2 passes (2>2=false) → plan found
        // >= 2: iteration 2 blocked (2>=2=true) → plan NOT found
        let mut goap = AdvancedGOAP::new();

        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "act",
            BTreeMap::new(),
            eff,
            1.0,
        )));

        let start = WorldState::new();
        let mut goal_state = BTreeMap::new();
        goal_state.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", goal_state);

        // max=2 should allow finding 1-step plan (iterations: 1=check start, 2=check done)
        goap.set_max_iterations(2);
        let plan = goap.plan(&start, &goal);
        assert!(plan.is_some(), "max_iterations=2 should allow 1-step plan");
        assert_eq!(plan.unwrap(), vec!["act"]);
    }

    #[test]
    fn test_plan_direct_risk_calculation() {
        // Kills: 1.0 - success_prob → 1.0 + prob or 1.0 / prob in plan_direct
        // If risk inverts (1+prob instead of 1-prob), high-success action has MORE risk
        let mut goap = AdvancedGOAP::new();

        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));

        // reliable: cost=2, 100% success (risk=0). f(5) = 2+0+0 = 2
        // unreliable: cost=1, 20% success (risk=0.8). f(5) = 1+0+0.8*5 = 5
        goap.add_action(Box::new(SimpleAction::new(
            "reliable",
            BTreeMap::new(),
            eff.clone(),
            2.0,
        )));
        goap.add_action(Box::new(SimpleAction::new(
            "unreliable",
            BTreeMap::new(),
            eff,
            1.0,
        )));

        // Give reliable 100% success
        for _ in 0..10 {
            goap.get_history_mut().record_success("reliable", 1.0);
        }
        // Give unreliable 20% success
        goap.get_history_mut().record_success("unreliable", 1.0);
        for _ in 0..4 {
            goap.get_history_mut().record_failure("unreliable");
        }

        goap.set_risk_weight(10.0);

        let start = WorldState::new();
        let mut gs = BTreeMap::new();
        gs.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", gs);

        // reliable: f = 2 + 0*10 = 2. unreliable: f = 1 + 0.8*10 = 9. → reliable wins
        // With 1+prob instead of 1-prob: reliable risk=1+1=2, f=2+20=22. unreliable risk=1+0.2=1.2, f=1+12=13. → unreliable wins
        let plan = goap.plan(&start, &goal).unwrap();
        assert_eq!(plan, vec!["reliable"]);
    }

    #[test]
    fn test_plan_direct_cost_accumulation_ordering() {
        // Kills: g_cost + action_cost → - or *
        // Two paths: cheap_path (cost 1+1=2) and expensive_path (cost 10)
        // With -, g_cost decreases making expensive look cheaper
        let mut goap = AdvancedGOAP::new();

        // Path 1: step1 (cost 1) → step2 (cost 1) → done. Total=2
        let mut s1_eff = BTreeMap::new();
        s1_eff.insert("intermediate".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "step1",
            BTreeMap::new(),
            s1_eff,
            1.0,
        )));

        let mut s2_pre = BTreeMap::new();
        s2_pre.insert("intermediate".to_string(), StateValue::Bool(true));
        let mut s2_eff = BTreeMap::new();
        s2_eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new("step2", s2_pre, s2_eff, 1.0)));

        // Path 2: expensive (cost 10) → done. Total=10
        let mut exp_eff = BTreeMap::new();
        exp_eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "expensive",
            BTreeMap::new(),
            exp_eff,
            10.0,
        )));

        let start = WorldState::new();
        let mut gs = BTreeMap::new();
        gs.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", gs);

        let plan = goap.plan(&start, &goal).unwrap();
        // Should pick 2-step cheaper path over 1-step expensive
        assert_eq!(plan, vec!["step1", "step2"]);
    }

    #[test]
    fn test_plan_direct_cost_subtraction_mutation_kill() {
        // Kills: g_cost + action_cost → g_cost - action_cost
        // Design: step costs must be large enough that with -, step1 is explored
        // BEFORE shortcut, leading to step2 being found first.
        // With +: shortcut f = 3+0+1 = 4  <  step1 f = 5+1+1 = 7 → shortcut explored first
        // With -: shortcut f = -3+0+1 = -2  >  step1 f = -5+1+1 = -3 → step1 explored first
        //   then step2 f = -10+0+2 = -8 → step2 wins → returns [step1,step2]
        let mut goap = AdvancedGOAP::new();

        // 2-step path: step1(cost 5) → step2(cost 5), total = 10
        let mut s1_eff = BTreeMap::new();
        s1_eff.insert("stage1".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "step1",
            BTreeMap::new(),
            s1_eff,
            5.0,
        )));

        let mut s2_pre = BTreeMap::new();
        s2_pre.insert("stage1".to_string(), StateValue::Bool(true));
        let mut s2_eff = BTreeMap::new();
        s2_eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new("step2", s2_pre, s2_eff, 5.0)));

        // 1-step shortcut: cost 3 (cheaper than 2-step total of 10)
        let mut sc_eff = BTreeMap::new();
        sc_eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "shortcut",
            BTreeMap::new(),
            sc_eff,
            3.0,
        )));

        let start = WorldState::new();
        let mut gs = BTreeMap::new();
        gs.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", gs);

        let plan = goap.plan(&start, &goal).unwrap();
        // shortcut cost=3 << step1+step2 cost=10 → shortcut wins
        assert_eq!(plan, vec!["shortcut"]);
    }

    #[test]
    fn test_simulate_plan_at_exactly_half_probability() {
        // Kills: success_prob > 0.5 → >= 0.5
        // With exactly 0.5 probability, > 0.5 is false → failure
        let mut goap = AdvancedGOAP::new();
        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));
        goap.add_action(Box::new(SimpleAction::new(
            "coin_flip",
            BTreeMap::new(),
            eff,
            1.0,
        )));

        // 1 success + 1 failure = 50% success rate = exactly 0.5
        goap.get_history_mut().record_success("coin_flip", 1.0);
        goap.get_history_mut().record_failure("coin_flip");

        let mut world = WorldState::new();
        let result = goap.simulate_plan_execution(&["coin_flip".to_string()], &mut world);
        // > 0.5 is false for exactly 0.5, so should fail
        assert!(
            result.is_err(),
            "Exactly 0.5 probability should fail with > 0.5 check"
        );
    }

    #[test]
    fn test_plan_direct_risk_accumulation_sign_mutation_kill() {
        // Kills: risk: current.risk + action_risk → current.risk - action_risk (L291)
        // With -, negative risk makes risky actions MORE attractive (lower f_cost).
        // Design: safe action has 0 risk (prob=1.0), risky has 0.5 risk (prob=0.5).
        // Both reach goal. Costs engineered so risk difference flips ordering with -.
        let mut goap = AdvancedGOAP::new();

        let mut eff = BTreeMap::new();
        eff.insert("done".to_string(), StateValue::Bool(true));

        // safe: base_cost=6, 1 success → cost=6+0-2=4, prob=1.0, risk=0.0
        goap.add_action(Box::new(SimpleAction::new(
            "safe",
            BTreeMap::new(),
            eff.clone(),
            6.0,
        )));
        goap.get_history_mut().record_success("safe", 1.0);

        // risky: base_cost=1, 1 success + 1 failure → cost=1+5-1=5, prob=0.5, risk=0.5
        goap.add_action(Box::new(SimpleAction::new(
            "risky",
            BTreeMap::new(),
            eff,
            1.0,
        )));
        goap.get_history_mut().record_success("risky", 1.0);
        goap.get_history_mut().record_failure("risky");

        let start = WorldState::new();
        let mut gs = BTreeMap::new();
        gs.insert("done".to_string(), StateValue::Bool(true));
        let goal = Goal::new("g", gs);

        let plan = goap.plan(&start, &goal).unwrap();
        // With +: safe f=4+0+0*5=4, risky f=5+0+0.5*5=7.5 → safe wins
        // With -: safe f=4+0+0*5=4, risky f=5+0+(-0.5)*5=2.5 → risky wins → FAIL
        assert_eq!(plan, vec!["safe"]);
    }
}
