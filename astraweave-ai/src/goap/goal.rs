use super::{StateValue, WorldState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Strategy for decomposing a goal into sub-goals
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecompositionStrategy {
    /// Sub-goals must be achieved in the order specified
    #[default]
    Sequential,
    /// Sub-goals can be pursued in any order or simultaneously
    Parallel,
    /// Any one sub-goal satisfying is sufficient for parent goal
    AnyOf,
    /// All sub-goals must be satisfied for parent goal to be satisfied
    AllOf,
}

/// Goal with priority, deadline, and hierarchical decomposition support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub name: String,
    pub desired_state: BTreeMap<String, StateValue>,
    pub priority: f32,
    pub deadline: Option<f32>, // Time by which goal must be achieved
    pub sub_goals: Vec<Goal>,  // Hierarchical goal decomposition
    pub decomposition_strategy: DecompositionStrategy,
    pub max_depth: usize, // Maximum recursion depth for hierarchical planning
}

impl Goal {
    /// Create a new goal with desired state conditions
    pub fn new(name: impl Into<String>, desired_state: BTreeMap<String, StateValue>) -> Self {
        Self {
            name: name.into(),
            desired_state,
            priority: 1.0,
            deadline: None,
            sub_goals: Vec::new(),
            decomposition_strategy: DecompositionStrategy::default(),
            max_depth: 5,
        }
    }

    /// Set priority (higher = more important)
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// Set deadline (time constraint for achieving this goal)
    pub fn with_deadline(mut self, deadline: f32) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Add sub-goals (hierarchical decomposition)
    pub fn with_sub_goals(mut self, sub_goals: Vec<Goal>) -> Self {
        self.sub_goals = sub_goals;
        self
    }

    /// Set decomposition strategy
    pub fn with_strategy(mut self, strategy: DecompositionStrategy) -> Self {
        self.decomposition_strategy = strategy;
        self
    }

    /// Set maximum recursion depth
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Check if goal is satisfied in current world state
    pub fn is_satisfied(&self, world: &WorldState) -> bool {
        world.satisfies(&self.desired_state)
    }

    /// Calculate urgency based on priority and deadline proximity
    /// Returns effective priority considering time pressure
    pub fn urgency(&self, current_time: f32) -> f32 {
        let base_urgency = self.priority;

        match self.deadline {
            Some(deadline) => {
                let time_remaining = (deadline - current_time).max(0.0);

                // As deadline approaches, urgency increases dramatically
                // Formula: priority * (1 + 10 / (time_remaining + 1))
                // At deadline: urgency ≈ priority * 11
                // Far from deadline: urgency ≈ priority
                base_urgency * (1.0 + 10.0 / (time_remaining + 1.0))
            }
            None => base_urgency,
        }
    }

    /// Get all conditions that are currently unsatisfied
    pub fn unmet_conditions(&self, world: &WorldState) -> Vec<(String, StateValue)> {
        self.desired_state
            .iter()
            .filter(|(key, target_value)| {
                world
                    .get(key)
                    .map(|current| !current.satisfies(target_value))
                    .unwrap_or(true) // Missing keys are unmet
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Calculate progress toward goal (0.0 = no progress, 1.0 = complete)
    pub fn progress(&self, world: &WorldState) -> f32 {
        if self.desired_state.is_empty() {
            return 1.0;
        }

        let met_conditions = self
            .desired_state
            .iter()
            .filter(|(key, target_value)| {
                world
                    .get(key)
                    .map(|current| current.satisfies(target_value))
                    .unwrap_or(false)
            })
            .count();

        met_conditions as f32 / self.desired_state.len() as f32
    }

    /// Check if goal is still achievable given current time and deadline
    pub fn is_achievable(&self, current_time: f32, estimated_completion_time: f32) -> bool {
        match self.deadline {
            Some(deadline) => current_time + estimated_completion_time <= deadline,
            None => true, // Goals without deadlines are always achievable
        }
    }

    /// Check if this goal should be decomposed into sub-goals
    pub fn should_decompose(&self, depth: usize) -> bool {
        !self.sub_goals.is_empty() && depth < self.max_depth
    }

    /// Decompose goal into sub-goals based on decomposition strategy
    /// Returns None if goal doesn't need decomposition
    pub fn decompose(&self) -> Option<Vec<Goal>> {
        if self.sub_goals.is_empty() {
            return None;
        }

        // Clone sub-goals and inherit priority if not explicitly set
        let sub_goals: Vec<Goal> = self
            .sub_goals
            .iter()
            .map(|sub| {
                let mut sub_clone = sub.clone();
                // Inherit parent priority if sub-goal has default priority
                if sub_clone.priority == 1.0 && self.priority != 1.0 {
                    sub_clone.priority = self.priority * 0.9; // Slightly lower than parent
                }
                sub_clone
            })
            .collect();

        Some(sub_goals)
    }

    /// Get effective sub-goals based on decomposition strategy
    /// For AnyOf, may return a subset; for others, returns all
    pub fn get_active_sub_goals(&self, world: &WorldState) -> Vec<Goal> {
        match self.decomposition_strategy {
            DecompositionStrategy::AnyOf => {
                // For AnyOf, only return unsatisfied sub-goals
                self.sub_goals
                    .iter()
                    .filter(|g| !g.is_satisfied(world))
                    .cloned()
                    .collect()
            }
            _ => self.sub_goals.clone(),
        }
    }

    /// Check if sub-goals satisfy parent goal based on decomposition strategy
    pub fn sub_goals_satisfy(&self, world: &WorldState) -> bool {
        if self.sub_goals.is_empty() {
            return false; // Can't be satisfied by non-existent sub-goals
        }

        match self.decomposition_strategy {
            DecompositionStrategy::Sequential | DecompositionStrategy::AllOf => {
                // All sub-goals must be satisfied
                self.sub_goals.iter().all(|g| g.is_satisfied(world))
            }
            DecompositionStrategy::AnyOf => {
                // At least one sub-goal must be satisfied
                self.sub_goals.iter().any(|g| g.is_satisfied(world))
            }
            DecompositionStrategy::Parallel => {
                // For parallel, check if all are satisfied (same as AllOf)
                self.sub_goals.iter().all(|g| g.is_satisfied(world))
            }
        }
    }

    /// Calculate depth of goal hierarchy
    pub fn depth(&self) -> usize {
        if self.sub_goals.is_empty() {
            1
        } else {
            1 + self.sub_goals.iter().map(|g| g.depth()).max().unwrap_or(0)
        }
    }

    /// Count total number of goals in hierarchy (including self)
    pub fn total_goal_count(&self) -> usize {
        1 + self
            .sub_goals
            .iter()
            .map(|g| g.total_goal_count())
            .sum::<usize>()
    }

    /// Flatten goal hierarchy into ordered sequence (depth-first)
    /// Respects decomposition strategy
    pub fn flatten(&self) -> Vec<Goal> {
        let mut result = Vec::new();

        match self.decomposition_strategy {
            DecompositionStrategy::Sequential => {
                // Add sub-goals in order
                for sub_goal in &self.sub_goals {
                    result.extend(sub_goal.flatten());
                }
                // Add this goal at the end
                result.push(self.without_sub_goals());
            }
            DecompositionStrategy::Parallel | DecompositionStrategy::AllOf => {
                // Add all sub-goals (order doesn't matter for parallel)
                for sub_goal in &self.sub_goals {
                    result.extend(sub_goal.flatten());
                }
                result.push(self.without_sub_goals());
            }
            DecompositionStrategy::AnyOf => {
                // For AnyOf, flatten but mark as alternatives
                // (Planner will need to try each until one succeeds)
                result.push(self.without_sub_goals());
                for sub_goal in &self.sub_goals {
                    result.extend(sub_goal.flatten());
                }
            }
        }

        result
    }

    /// Create a copy of this goal without sub-goals (leaf node)
    fn without_sub_goals(&self) -> Goal {
        Goal {
            name: self.name.clone(),
            desired_state: self.desired_state.clone(),
            priority: self.priority,
            deadline: self.deadline,
            sub_goals: Vec::new(),
            decomposition_strategy: self.decomposition_strategy,
            max_depth: self.max_depth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_creation() {
        let mut desired = BTreeMap::new();
        desired.insert("health".to_string(), StateValue::Int(100));

        let goal = Goal::new("stay_alive", desired.clone())
            .with_priority(10.0)
            .with_deadline(30.0);

        assert_eq!(goal.name, "stay_alive");
        assert_eq!(goal.priority, 10.0);
        assert_eq!(goal.deadline, Some(30.0));
        assert_eq!(goal.desired_state, desired);
    }

    #[test]
    fn test_goal_is_satisfied() {
        let mut desired = BTreeMap::new();
        desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        desired.insert("ammo".to_string(), StateValue::IntRange(5, 100));

        let goal = Goal::new("ready_for_combat", desired);

        let mut world = WorldState::new();
        world.set("weapon_equipped", StateValue::Bool(true));
        world.set("ammo", StateValue::Int(20));

        assert!(goal.is_satisfied(&world));

        let mut world_not_ready = WorldState::new();
        world_not_ready.set("weapon_equipped", StateValue::Bool(false));
        world_not_ready.set("ammo", StateValue::Int(20));

        assert!(!goal.is_satisfied(&world_not_ready));
    }

    #[test]
    fn test_urgency_without_deadline() {
        let goal = Goal::new("explore", BTreeMap::new()).with_priority(5.0);

        assert_eq!(goal.urgency(0.0), 5.0);
        assert_eq!(goal.urgency(100.0), 5.0); // No deadline, constant urgency
    }

    #[test]
    fn test_urgency_with_deadline() {
        let goal = Goal::new("escape", BTreeMap::new())
            .with_priority(5.0)
            .with_deadline(10.0);

        let urgency_far = goal.urgency(0.0); // 10 seconds remaining
        let urgency_near = goal.urgency(9.0); // 1 second remaining
        let urgency_at = goal.urgency(10.0); // At deadline

        assert!(urgency_near > urgency_far);
        assert!(urgency_at > urgency_near);
    }

    #[test]
    fn test_urgency_exact_values() {
        // Formula: priority * (1.0 + 10.0 / (time_remaining + 1.0))
        // where time_remaining = (deadline - current_time).max(0.0)
        let goal = Goal::new("timed", BTreeMap::new())
            .with_priority(3.0)
            .with_deadline(20.0);

        // At time=10: remaining=10, urgency = 3.0 * (1.0 + 10.0 / 11.0) = 3.0 * 1.909... ≈ 5.727
        let urgency = goal.urgency(10.0);
        let expected = 3.0 * (1.0 + 10.0 / 11.0);
        assert!(
            (urgency - expected).abs() < 0.001,
            "urgency={urgency}, expected={expected}"
        );

        // At deadline (time=20): remaining=0, urgency = 3.0 * (1 + 10/1) = 33.0
        let urgency_at = goal.urgency(20.0);
        assert!(
            (urgency_at - 33.0).abs() < 0.001,
            "urgency at deadline={urgency_at}, expected=33.0"
        );

        // Past deadline (time=25): remaining=(20-25).max(0)=0, urgency = 33.0
        let urgency_past = goal.urgency(25.0);
        assert!(
            (urgency_past - 33.0).abs() < 0.001,
            "urgency past deadline={urgency_past}"
        );
    }

    #[test]
    fn test_unmet_conditions() {
        let mut desired = BTreeMap::new();
        desired.insert("health".to_string(), StateValue::Int(100));
        desired.insert("ammo".to_string(), StateValue::Int(30));
        desired.insert("in_cover".to_string(), StateValue::Bool(true));

        let goal = Goal::new("prepared", desired);

        let mut world = WorldState::new();
        world.set("health", StateValue::Int(50)); // Unmet
        world.set("ammo", StateValue::Int(30)); // Met
                                                // in_cover missing                       // Unmet

        let unmet = goal.unmet_conditions(&world);
        assert_eq!(unmet.len(), 2);
        assert!(unmet.iter().any(|(k, _)| k == "health"));
        assert!(unmet.iter().any(|(k, _)| k == "in_cover"));
    }

    #[test]
    fn test_progress() {
        let mut desired = BTreeMap::new();
        desired.insert("health".to_string(), StateValue::Int(100));
        desired.insert("ammo".to_string(), StateValue::Int(30));
        desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));

        let goal = Goal::new("combat_ready", desired);

        let mut world = WorldState::new();
        assert_eq!(goal.progress(&world), 0.0); // Nothing met

        world.set("health", StateValue::Int(100));
        assert!((goal.progress(&world) - 0.333).abs() < 0.01); // 1/3 met

        world.set("ammo", StateValue::Int(30));
        assert!((goal.progress(&world) - 0.666).abs() < 0.01); // 2/3 met

        world.set("weapon_equipped", StateValue::Bool(true));
        assert_eq!(goal.progress(&world), 1.0); // All met
    }

    #[test]
    fn test_is_achievable() {
        let goal = Goal::new("timed_objective", BTreeMap::new()).with_deadline(100.0);

        assert!(goal.is_achievable(50.0, 30.0)); // 50 + 30 <= 100
        assert!(!goal.is_achievable(80.0, 30.0)); // 80 + 30 > 100

        let no_deadline_goal = Goal::new("open_ended", BTreeMap::new());
        assert!(no_deadline_goal.is_achievable(1000.0, 1000.0)); // Always achievable
    }

    #[test]
    fn test_hierarchical_goals() {
        let mut sub1_desired = BTreeMap::new();
        sub1_desired.insert("has_weapon".to_string(), StateValue::Bool(true));
        let sub1 = Goal::new("find_weapon", sub1_desired);

        let mut sub2_desired = BTreeMap::new();
        sub2_desired.insert("weapon_equipped".to_string(), StateValue::Bool(true));
        let sub2 = Goal::new("equip_weapon", sub2_desired);

        let mut main_desired = BTreeMap::new();
        main_desired.insert("enemy_defeated".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("defeat_enemy", main_desired).with_sub_goals(vec![sub1, sub2]);

        assert_eq!(main_goal.sub_goals.len(), 2);
    }

    #[test]
    fn test_flatten_goals() {
        let mut sub_desired = BTreeMap::new();
        sub_desired.insert("prerequisite".to_string(), StateValue::Bool(true));
        let sub_goal = Goal::new("prepare", sub_desired);

        let mut main_desired = BTreeMap::new();
        main_desired.insert("objective".to_string(), StateValue::Bool(true));
        let main_goal = Goal::new("complete", main_desired).with_sub_goals(vec![sub_goal]);

        let flattened = main_goal.flatten();
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].name, "prepare"); // Sub-goal first
        assert_eq!(flattened[1].name, "complete"); // Main goal second
    }

    #[test]
    fn test_goal_comparison_by_urgency() {
        let goal1 = Goal::new("low_priority", BTreeMap::new()).with_priority(2.0);

        let goal2 = Goal::new("high_priority", BTreeMap::new()).with_priority(8.0);

        let goal3 = Goal::new("urgent_deadline", BTreeMap::new())
            .with_priority(3.0)
            .with_deadline(1.0); // Very close deadline

        let current_time = 0.5;

        assert!(goal2.urgency(current_time) > goal1.urgency(current_time));
        assert!(goal3.urgency(current_time) > goal2.urgency(current_time)); // Deadline wins
    }

    #[test]
    fn test_decomposition_strategy() {
        let goal =
            Goal::new("test", BTreeMap::new()).with_strategy(DecompositionStrategy::Parallel);

        assert_eq!(goal.decomposition_strategy, DecompositionStrategy::Parallel);
    }

    #[test]
    fn test_should_decompose() {
        let goal_no_subs = Goal::new("simple", BTreeMap::new());
        assert!(!goal_no_subs.should_decompose(0));

        let sub_goal = Goal::new("sub", BTreeMap::new());
        let goal_with_subs = Goal::new("complex", BTreeMap::new()).with_sub_goals(vec![sub_goal]);

        assert!(goal_with_subs.should_decompose(0));
        assert!(goal_with_subs.should_decompose(4));
        assert!(!goal_with_subs.should_decompose(5)); // At max depth
        assert!(!goal_with_subs.should_decompose(10)); // Beyond max depth
    }

    #[test]
    fn test_decompose_priority_inheritance() {
        let sub1 = Goal::new("sub1", BTreeMap::new()); // Default priority 1.0
        let sub2 = Goal::new("sub2", BTreeMap::new()).with_priority(5.0); // Explicit priority

        let parent = Goal::new("parent", BTreeMap::new())
            .with_priority(10.0)
            .with_sub_goals(vec![sub1, sub2]);

        let decomposed = parent.decompose().unwrap();
        assert_eq!(decomposed.len(), 2);
        assert_eq!(decomposed[0].priority, 9.0); // Inherited from parent (10.0 * 0.9)
        assert_eq!(decomposed[1].priority, 5.0); // Kept explicit priority
    }

    #[test]
    fn test_get_active_sub_goals_any_of() {
        let mut world = WorldState::new();
        world.set("condition_a", StateValue::Bool(true));

        let mut desired_a = BTreeMap::new();
        desired_a.insert("condition_a".to_string(), StateValue::Bool(true));
        let sub_a = Goal::new("sub_a", desired_a);

        let mut desired_b = BTreeMap::new();
        desired_b.insert("condition_b".to_string(), StateValue::Bool(true));
        let sub_b = Goal::new("sub_b", desired_b);

        let parent = Goal::new("parent", BTreeMap::new())
            .with_strategy(DecompositionStrategy::AnyOf)
            .with_sub_goals(vec![sub_a, sub_b]);

        let active = parent.get_active_sub_goals(&world);
        assert_eq!(active.len(), 1); // Only sub_b (sub_a is satisfied)
        assert_eq!(active[0].name, "sub_b");
    }

    #[test]
    fn test_sub_goals_satisfy_sequential() {
        let mut world = WorldState::new();
        world.set("step1", StateValue::Bool(true));
        world.set("step2", StateValue::Bool(true));

        let mut desired1 = BTreeMap::new();
        desired1.insert("step1".to_string(), StateValue::Bool(true));
        let sub1 = Goal::new("sub1", desired1);

        let mut desired2 = BTreeMap::new();
        desired2.insert("step2".to_string(), StateValue::Bool(true));
        let sub2 = Goal::new("sub2", desired2);

        let parent = Goal::new("parent", BTreeMap::new())
            .with_strategy(DecompositionStrategy::Sequential)
            .with_sub_goals(vec![sub1, sub2]);

        assert!(parent.sub_goals_satisfy(&world)); // All satisfied

        let mut partial_world = WorldState::new();
        partial_world.set("step1", StateValue::Bool(true));
        assert!(!parent.sub_goals_satisfy(&partial_world)); // Not all satisfied
    }

    #[test]
    fn test_sub_goals_satisfy_any_of() {
        let mut world = WorldState::new();
        world.set("option_a", StateValue::Bool(true));

        let mut desired_a = BTreeMap::new();
        desired_a.insert("option_a".to_string(), StateValue::Bool(true));
        let sub_a = Goal::new("sub_a", desired_a);

        let mut desired_b = BTreeMap::new();
        desired_b.insert("option_b".to_string(), StateValue::Bool(true));
        let sub_b = Goal::new("sub_b", desired_b);

        let parent = Goal::new("parent", BTreeMap::new())
            .with_strategy(DecompositionStrategy::AnyOf)
            .with_sub_goals(vec![sub_a, sub_b]);

        assert!(parent.sub_goals_satisfy(&world)); // At least one satisfied
    }

    #[test]
    fn test_goal_depth() {
        let leaf = Goal::new("leaf", BTreeMap::new());
        assert_eq!(leaf.depth(), 1);

        let sub1 = Goal::new("sub1", BTreeMap::new());
        let sub2 = Goal::new("sub2", BTreeMap::new());
        let parent = Goal::new("parent", BTreeMap::new()).with_sub_goals(vec![sub1, sub2]);
        assert_eq!(parent.depth(), 2);

        let subsub = Goal::new("subsub", BTreeMap::new());
        let sub = Goal::new("sub", BTreeMap::new()).with_sub_goals(vec![subsub]);
        let root = Goal::new("root", BTreeMap::new()).with_sub_goals(vec![sub]);
        assert_eq!(root.depth(), 3);
    }

    #[test]
    fn test_total_goal_count() {
        let leaf = Goal::new("leaf", BTreeMap::new());
        assert_eq!(leaf.total_goal_count(), 1);

        let sub1 = Goal::new("sub1", BTreeMap::new());
        let sub2 = Goal::new("sub2", BTreeMap::new());
        let parent = Goal::new("parent", BTreeMap::new()).with_sub_goals(vec![sub1, sub2]);
        assert_eq!(parent.total_goal_count(), 3); // parent + 2 subs

        let subsub1 = Goal::new("subsub1", BTreeMap::new());
        let subsub2 = Goal::new("subsub2", BTreeMap::new());
        let sub = Goal::new("sub", BTreeMap::new()).with_sub_goals(vec![subsub1, subsub2]);
        let root = Goal::new("root", BTreeMap::new()).with_sub_goals(vec![sub]);
        assert_eq!(root.total_goal_count(), 4); // root + sub + 2 subsubs
    }

    #[test]
    fn test_flatten_sequential() {
        let sub1 = Goal::new("sub1", BTreeMap::new());
        let sub2 = Goal::new("sub2", BTreeMap::new());
        let parent = Goal::new("parent", BTreeMap::new())
            .with_strategy(DecompositionStrategy::Sequential)
            .with_sub_goals(vec![sub1, sub2]);

        let flattened = parent.flatten();
        assert_eq!(flattened.len(), 3);
        assert_eq!(flattened[0].name, "sub1");
        assert_eq!(flattened[1].name, "sub2");
        assert_eq!(flattened[2].name, "parent");
    }

    #[test]
    fn test_flatten_any_of() {
        let sub1 = Goal::new("sub1", BTreeMap::new());
        let sub2 = Goal::new("sub2", BTreeMap::new());
        let parent = Goal::new("parent", BTreeMap::new())
            .with_strategy(DecompositionStrategy::AnyOf)
            .with_sub_goals(vec![sub1, sub2]);

        let flattened = parent.flatten();
        assert_eq!(flattened.len(), 3);
        assert_eq!(flattened[0].name, "parent"); // Parent first for AnyOf
        assert_eq!(flattened[1].name, "sub1");
        assert_eq!(flattened[2].name, "sub2");
    }
}
