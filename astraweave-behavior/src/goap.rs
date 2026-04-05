//! GOAP (Goal-Oriented Action Planning) planner
//!
//! Implements A* planning over symbolic world states with deterministic execution.

#[cfg(feature = "profiling")]
use astraweave_profiling::span;

use crate::interner::intern;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::rc::Rc;

/// Symbolic world state represented as a deterministic map
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorldState {
    pub facts: BTreeMap<u32, bool>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            facts: BTreeMap::new(),
        }
    }

    pub fn from_facts(facts: &[(&str, bool)]) -> Self {
        let mut state = Self::new();
        for (key, value) in facts {
            state.set(key, *value);
        }
        state
    }

    pub fn set(&mut self, key: &str, value: bool) {
        self.facts.insert(intern(key), value);
    }

    pub fn get(&self, key: &str) -> Option<bool> {
        self.facts.get(&intern(key)).copied()
    }

    pub fn satisfies(&self, other: &WorldState) -> bool {
        for (key, &value) in &other.facts {
            if self.facts.get(key).copied() != Some(value) {
                return false;
            }
        }
        true
    }

    /// Apply effects of an action to this state
    pub fn apply(&mut self, effects: &WorldState) {
        for (&key, &value) in &effects.facts {
            self.facts.insert(key, value);
        }
    }

    /// Distance metric for A* heuristic (count of unsatisfied facts)
    pub fn distance_to(&self, goal: &WorldState) -> usize {
        goal.facts
            .iter()
            .filter(|(key, &value)| self.facts.get(key).copied() != Some(value))
            .count()
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// A GOAP action with preconditions and effects
#[derive(Debug, Clone)]
pub struct GoapAction {
    pub name: String,
    pub cost: f32,
    pub preconditions: WorldState,
    pub effects: WorldState,
}

impl GoapAction {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cost: 1.0,
            preconditions: WorldState::new(),
            effects: WorldState::new(),
        }
    }

    pub fn with_cost(mut self, cost: f32) -> Self {
        self.cost = cost;
        self
    }

    pub fn with_precondition(mut self, key: &str, value: bool) -> Self {
        self.preconditions.set(key, value);
        self
    }

    pub fn with_effect(mut self, key: &str, value: bool) -> Self {
        self.effects.set(key, value);
        self
    }

    /// Check if this action can be applied in the given state
    pub fn can_apply(&self, state: &WorldState) -> bool {
        state.satisfies(&self.preconditions)
    }

    /// Apply this action's effects to a state (returns new state)
    pub fn apply(&self, state: &WorldState) -> WorldState {
        let mut new_state = state.clone();
        new_state.apply(&self.effects);
        new_state
    }
}

/// A GOAP goal with desired state and priority
#[derive(Debug, Clone)]
pub struct GoapGoal {
    pub name: String,
    pub desired_state: WorldState,
    pub priority: f32,
}

impl GoapGoal {
    pub fn new(name: impl Into<String>, desired_state: WorldState) -> Self {
        Self {
            name: name.into(),
            desired_state,
            priority: 1.0,
        }
    }

    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if the given state satisfies this goal
    pub fn is_satisfied(&self, state: &WorldState) -> bool {
        state.satisfies(&self.desired_state)
    }
}

/// A* search node for planning
#[derive(Debug, Clone)]
struct PlanNode {
    state: WorldState,
    parent: Option<Rc<PlanNode>>,
    action: Option<GoapAction>,
    g_cost: f32,
    h_cost: f32,
    depth: usize,
}

impl PlanNode {
    fn f_cost(&self) -> f32 {
        self.g_cost + self.h_cost
    }
}

impl PartialEq for PlanNode {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
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
        // Reverse ordering for min-heap (lower f_cost is better)
        other
            .f_cost()
            .partial_cmp(&self.f_cost())
            .unwrap_or(Ordering::Equal)
            // Deterministic tie-breaking: fewer actions first
            .then_with(|| other.depth.cmp(&self.depth))
            // Then by last action name (lexicographic) - comparing other to self for correct min-heap order
            .then_with(|| match (&other.action, &self.action) {
                (Some(a), Some(b)) => a.name.cmp(&b.name),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
    }
}

/// GOAP planner using A* search
pub struct GoapPlanner {
    max_iterations: usize,
}

impl GoapPlanner {
    pub fn new() -> Self {
        Self {
            max_iterations: 1000,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Plan a sequence of actions to achieve the goal
    /// Returns None if no plan found within max_iterations
    pub fn plan(
        &self,
        current_state: &WorldState,
        goal: &GoapGoal,
        available_actions: &[GoapAction],
    ) -> Option<Vec<GoapAction>> {
        #[cfg(feature = "profiling")]
        span!("AI::GOAP::plan");

        // Early exit if goal already satisfied
        if goal.is_satisfied(current_state) {
            return Some(Vec::new());
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = BTreeSet::new();

        // Start node
        let start_node = Rc::new(PlanNode {
            state: current_state.clone(),
            parent: None,
            action: None,
            g_cost: 0.0,
            h_cost: current_state.distance_to(&goal.desired_state) as f32,
            depth: 0,
        });

        open_set.push(start_node);

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                return None; // Exceeded iteration limit
            }

            // Goal check
            if goal.is_satisfied(&current.state) {
                // Reconstruct path
                let mut path = Vec::new();
                let mut curr = current;
                while let Some(action) = &curr.action {
                    path.push(action.clone());
                    if let Some(parent) = &curr.parent {
                        curr = parent.clone();
                    } else {
                        break;
                    }
                }
                path.reverse();
                return Some(path);
            }

            // Skip if already explored
            if closed_set.contains(&current.state) {
                continue;
            }
            closed_set.insert(current.state.clone());

            // Expand neighbors (apply each applicable action)
            for action in available_actions {
                if action.can_apply(&current.state) {
                    let new_state = action.apply(&current.state);

                    // Skip if already in closed set
                    if closed_set.contains(&new_state) {
                        continue;
                    }

                    let new_g_cost = current.g_cost + action.cost;
                    let new_h_cost = new_state.distance_to(&goal.desired_state) as f32;

                    let new_node = Rc::new(PlanNode {
                        state: new_state,
                        parent: Some(current.clone()),
                        action: Some(action.clone()),
                        g_cost: new_g_cost,
                        h_cost: new_h_cost,
                        depth: current.depth + 1,
                    });

                    open_set.push(new_node);
                }
            }
        }

        None // No plan found
    }
}

impl Default for GoapPlanner {
    fn default() -> Self {
        Self::new()
    }
}

/// GOAP plan execution state
#[derive(Debug, Clone)]
pub struct GoapPlan {
    pub actions: VecDeque<GoapAction>,
    pub current_action: Option<GoapAction>,
    pub completed: Vec<GoapAction>,
}

impl GoapPlan {
    pub fn new(actions: Vec<GoapAction>) -> Self {
        let mut queue: VecDeque<_> = actions.into();
        let current_action = queue.pop_front();
        Self {
            actions: queue,
            current_action,
            completed: Vec::new(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_action.is_none() && self.actions.is_empty()
    }

    /// Advance to next action (call when current action completes)
    pub fn advance(&mut self) {
        if let Some(action) = self.current_action.take() {
            self.completed.push(action);
        }
        self.current_action = self.actions.pop_front();
    }

    /// Invalidate plan (forces replan)
    pub fn invalidate(&mut self) {
        self.actions.clear();
        self.current_action = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_state_satisfies() {
        let mut state1 = WorldState::new();
        state1.set("has_food", true);
        state1.set("is_hungry", false);

        let mut state2 = WorldState::new();
        state2.set("has_food", true);

        assert!(state1.satisfies(&state2));
        assert!(!state2.satisfies(&state1));
    }

    #[test]
    fn test_action_application() {
        let mut current = WorldState::new();
        current.set("has_herbs", true);
        current.set("has_food", false);

        let action = GoapAction::new("craft_food")
            .with_precondition("has_herbs", true)
            .with_effect("has_food", true)
            .with_effect("has_herbs", false);

        assert!(action.can_apply(&current));

        let new_state = action.apply(&current);
        assert_eq!(new_state.get("has_food"), Some(true));
        assert_eq!(new_state.get("has_herbs"), Some(false));
    }

    #[test]
    fn test_simple_plan() {
        let actions = vec![
            GoapAction::new("gather_herbs")
                .with_cost(5.0)
                .with_effect("has_herbs", true),
            GoapAction::new("craft_food")
                .with_cost(3.0)
                .with_precondition("has_herbs", true)
                .with_effect("has_food", true),
        ];

        let current_state = WorldState::new();
        let goal = GoapGoal::new("get_food", WorldState::from_facts(&[("has_food", true)]));

        let planner = GoapPlanner::new();
        let plan = planner.plan(&current_state, &goal, &actions).unwrap();

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].name, "gather_herbs");
        assert_eq!(plan[1].name, "craft_food");
    }

    #[test]
    fn test_plan_optimality() {
        // Two paths: expensive direct vs cheap chained
        let actions = vec![
            GoapAction::new("expensive_direct")
                .with_cost(20.0)
                .with_effect("has_food", true),
            GoapAction::new("gather_herbs")
                .with_cost(5.0)
                .with_effect("has_herbs", true),
            GoapAction::new("craft_food")
                .with_cost(3.0)
                .with_precondition("has_herbs", true)
                .with_effect("has_food", true),
        ];

        let current_state = WorldState::new();
        let goal = GoapGoal::new("get_food", WorldState::from_facts(&[("has_food", true)]));

        let planner = GoapPlanner::new();
        let plan = planner.plan(&current_state, &goal, &actions).unwrap();

        // Should choose cheaper path (gather + craft = 8.0 < direct = 20.0)
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].name, "gather_herbs");
        assert_eq!(plan[1].name, "craft_food");
    }

    #[test]
    fn test_deterministic_planning() {
        let actions = vec![
            GoapAction::new("action_a")
                .with_cost(5.0)
                .with_effect("state_x", true),
            GoapAction::new("action_b")
                .with_cost(5.0)
                .with_effect("state_x", true),
        ];

        let current_state = WorldState::new();
        let goal = GoapGoal::new("goal", WorldState::from_facts(&[("state_x", true)]));

        let planner = GoapPlanner::new();

        // Run multiple times, should get same result (deterministic tie-breaking)
        let plan1 = planner.plan(&current_state, &goal, &actions).unwrap();
        let plan2 = planner.plan(&current_state, &goal, &actions).unwrap();
        let plan3 = planner.plan(&current_state, &goal, &actions).unwrap();

        assert_eq!(plan1.len(), plan2.len());
        assert_eq!(plan1.len(), plan3.len());
        assert_eq!(plan1[0].name, plan2[0].name);
        assert_eq!(plan1[0].name, plan3[0].name);

        // Should pick "action_a" (lexicographically first)
        assert_eq!(plan1[0].name, "action_a");
    }

    #[test]
    fn test_no_plan_found() {
        let actions = vec![GoapAction::new("useless_action")
            .with_precondition("impossible", true)
            .with_effect("has_food", true)];

        let current_state = WorldState::new();
        let goal = GoapGoal::new("get_food", WorldState::from_facts(&[("has_food", true)]));

        let planner = GoapPlanner::new();
        let plan = planner.plan(&current_state, &goal, &actions);

        assert!(plan.is_none());
    }

    #[test]
    fn test_already_satisfied_goal() {
        let mut current_state = WorldState::new();
        current_state.set("has_food", true);

        let goal = GoapGoal::new("get_food", WorldState::from_facts(&[("has_food", true)]));

        let planner = GoapPlanner::new();
        let plan = planner.plan(&current_state, &goal, &[]).unwrap();

        assert!(plan.is_empty());
    }

    #[test]
    fn test_plan_execution() {
        let actions = vec![
            GoapAction::new("step1"),
            GoapAction::new("step2"),
            GoapAction::new("step3"),
        ];

        let mut plan = GoapPlan::new(actions);

        assert_eq!(plan.current_action.as_ref().unwrap().name, "step1");
        assert!(!plan.is_complete());

        plan.advance();
        assert_eq!(plan.current_action.as_ref().unwrap().name, "step2");
        assert_eq!(plan.completed.len(), 1);

        plan.advance();
        assert_eq!(plan.current_action.as_ref().unwrap().name, "step3");
        assert_eq!(plan.completed.len(), 2);

        plan.advance();
        assert!(plan.is_complete());
        assert_eq!(plan.completed.len(), 3);
    }
    #[test]
    fn test_max_iterations_limit() {
        // Create a large chain of actions
        let actions: Vec<GoapAction> = (0..100)
            .map(|i| {
                GoapAction::new(format!("step_{}", i))
                    .with_precondition(&format!("state_{}", i), true)
                    .with_effect(&format!("state_{}", i + 1), true)
            })
            .collect();

        let mut current_state = WorldState::new();
        current_state.set("state_0", true);

        let goal = GoapGoal::new("reach_end", WorldState::from_facts(&[("state_100", true)]));

        // Set a low iteration limit
        let planner = GoapPlanner::new().with_max_iterations(10);
        let plan = planner.plan(&current_state, &goal, &actions);

        // Should fail due to iteration limit
        assert!(plan.is_none());
    }

    // ═══════════════════════════════════════════════════════════════
    // MUTATION REMEDIATION TESTS — targets missed mutants in goap.rs
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_distance_to_returns_exact_count() {
        // Targets: goap.rs:60 replace distance_to -> usize with 0/1
        // and goap.rs:62 replace != with ==
        let s = WorldState::from_facts(&[("a", true), ("b", false), ("c", true)]);
        let goal = WorldState::from_facts(&[("a", true), ("b", true), ("c", false)]);
        // "a" matches, "b" doesn't (false vs true), "c" doesn't (true vs false) → 2
        assert_eq!(s.distance_to(&goal), 2);

        // All matching → 0
        let s2 = WorldState::from_facts(&[("x", true)]);
        let g2 = WorldState::from_facts(&[("x", true)]);
        assert_eq!(s2.distance_to(&g2), 0);

        // All different → 3
        let s3 = WorldState::from_facts(&[("a", false), ("b", false), ("c", false)]);
        let g3 = WorldState::from_facts(&[("a", true), ("b", true), ("c", true)]);
        assert_eq!(s3.distance_to(&g3), 3);
    }

    #[test]
    fn mutation_plan_node_f_cost_is_sum() {
        // Targets: goap.rs:161 replace + with - in PlanNode::f_cost
        // We verify indirectly: planner should find cheapest path
        let actions = vec![
            GoapAction::new("cheap")
                .with_cost(1.0)
                .with_precondition("start", true)
                .with_effect("goal", true),
            GoapAction::new("expensive")
                .with_cost(100.0)
                .with_precondition("start", true)
                .with_effect("goal", true),
        ];
        let state = WorldState::from_facts(&[("start", true)]);
        let goal = GoapGoal::new("g", WorldState::from_facts(&[("goal", true)]));
        let planner = GoapPlanner::new();
        let plan = planner.plan(&state, &goal, &actions).unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].name, "cheap", "Should pick cheapest action");
    }

    #[test]
    fn mutation_f_cost_sum_with_high_heuristic() {
        // Targets: goap.rs:161 replace + with - in PlanNode::f_cost
        //
        // With g+h: "useful" (f=1+0=1) beats distracts (f=0.1+5=5.1) -> found in ~2 iters
        // With g-h: distracts (f=0.1-5=-4.9) beat "useful" (f=1-0=1) -> exponential
        //           distraction blowup exhausts max_iterations before useful is popped
        let mut actions = vec![GoapAction::new("useful")
            .with_cost(1.0)
            .with_effect("g1", true)
            .with_effect("g2", true)
            .with_effect("g3", true)
            .with_effect("g4", true)
            .with_effect("g5", true)];

        // 6 distraction actions -- set irrelevant facts, h stays 5
        for i in 0..6 {
            actions.push(
                GoapAction::new(format!("distract_{}", i))
                    .with_cost(0.1)
                    .with_effect(&format!("junk_{}", i), true),
            );
        }

        let state = WorldState::new();
        let goal = GoapGoal::new(
            "achieve_all",
            WorldState::from_facts(&[
                ("g1", true),
                ("g2", true),
                ("g3", true),
                ("g4", true),
                ("g5", true),
            ]),
        );

        let planner = GoapPlanner::new().with_max_iterations(15);
        let plan = planner.plan(&state, &goal, &actions);

        assert!(
            plan.is_some(),
            "Planner MUST find plan with correct f_cost = g + h"
        );
        let plan = plan.unwrap();
        assert_eq!(plan.len(), 1, "Should find single-step plan via 'useful'");
        assert_eq!(plan[0].name, "useful");
    }

    #[test]
    fn mutation_plan_node_eq_checks_state() {
        // Targets: goap.rs:167 replace eq -> bool with true/false
        // and goap.rs:167 replace == with !=
        // Indirectly verified: planner deduplication uses eq
        let actions = vec![
            GoapAction::new("a1")
                .with_precondition("s", true)
                .with_effect("mid", true),
            GoapAction::new("a2")
                .with_precondition("mid", true)
                .with_effect("done", true),
        ];
        let state = WorldState::from_facts(&[("s", true)]);
        let goal = GoapGoal::new("g", WorldState::from_facts(&[("done", true)]));
        let planner = GoapPlanner::new();
        let plan = planner.plan(&state, &goal, &actions).unwrap();
        assert_eq!(plan.len(), 2);
    }

    #[test]
    fn mutation_planner_iteration_guard() {
        // Targets: goap.rs:250 replace > with ==/>=
        // If > becomes ==, planner would only stop at exactly max_iterations, not above
        let planner = GoapPlanner::new().with_max_iterations(1);
        let state = WorldState::from_facts(&[("a", true)]);
        let goal = GoapGoal::new(
            "g",
            WorldState::from_facts(&[("a", true), ("b", true), ("c", true)]),
        );
        let actions = vec![
            GoapAction::new("x")
                .with_precondition("a", true)
                .with_effect("b", true)
                .with_cost(1.0),
            GoapAction::new("y")
                .with_precondition("b", true)
                .with_effect("c", true)
                .with_cost(1.0),
        ];
        // With max_iterations=1, should fail
        assert!(planner.plan(&state, &goal, &actions).is_none());
    }

    #[test]
    fn mutation_plan_cost_addition() {
        // Targets: goap.rs:296 replace + with * in GoapPlanner::plan (g_cost += action.cost)
        let actions = vec![
            GoapAction::new("step1")
                .with_cost(2.0)
                .with_precondition("s", true)
                .with_effect("m", true),
            GoapAction::new("step2")
                .with_cost(3.0)
                .with_precondition("m", true)
                .with_effect("g", true),
        ];
        let state = WorldState::from_facts(&[("s", true)]);
        let goal = GoapGoal::new("g", WorldState::from_facts(&[("g", true)]));
        let planner = GoapPlanner::new();
        let plan = planner.plan(&state, &goal, &actions).unwrap();
        // Plan should find both steps
        assert_eq!(plan.len(), 2);
    }

    #[test]
    fn mutation_goap_plan_is_complete_conjunction() {
        // Targets: goap.rs:334 replace && with || in GoapPlan::is_complete
        let actions = vec![GoapAction::new("step1")];
        let mut plan = GoapPlan::new(actions);
        // With current_action=Some and actions empty, is_complete should be false
        assert!(
            !plan.is_complete(),
            "Plan with current action should not be complete"
        );
        plan.advance();
        // Now current_action=None and actions empty → complete
        assert!(
            plan.is_complete(),
            "After advancing past last, should be complete"
        );
    }

    #[test]
    fn mutation_goap_plan_invalidate_clears() {
        // Targets: goap.rs:347 replace invalidate with ()
        let actions = vec![
            GoapAction::new("s1"),
            GoapAction::new("s2"),
            GoapAction::new("s3"),
        ];
        let mut plan = GoapPlan::new(actions);
        assert!(!plan.is_complete());

        plan.invalidate();
        // After invalidation, plan should be complete (nothing left to do)
        assert!(
            plan.is_complete(),
            "Invalidated plan should be complete (cleared)"
        );
    }
}
