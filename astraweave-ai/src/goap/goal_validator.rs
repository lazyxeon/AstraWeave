use super::goal_authoring::{GoalDefinition, StateValueDef};
use std::collections::{HashMap, HashSet};

/// Validation error with location and severity
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub severity: Severity,
    pub message: String,
    pub field: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl ValidationError {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Validation result containing errors and warnings
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
    pub info: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, error: ValidationError) {
        match error.severity {
            Severity::Error => self.errors.push(error),
            Severity::Warning => self.warnings.push(error),
            Severity::Info => self.info.push(error),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len() + self.info.len()
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
    }
}

/// Main goal validator
pub struct GoalValidator {
    known_state_variables: HashSet<String>,
    strict_mode: bool,
}

impl GoalValidator {
    pub fn new() -> Self {
        Self {
            known_state_variables: Self::default_state_variables(),
            strict_mode: false,
        }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub fn add_known_state_variable(&mut self, var: impl Into<String>) {
        self.known_state_variables.insert(var.into());
    }

    /// Validate a goal definition
    pub fn validate(&self, goal: &GoalDefinition) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Schema validation
        self.validate_schema(goal, &mut result);

        // Semantic validation
        self.validate_semantics(goal, &mut result);

        // Circular dependency check
        self.check_circular_dependencies(goal, &mut result);

        // Complexity analysis
        self.analyze_complexity(goal, &mut result);

        result
    }

    /// Schema validation: check structure and types
    fn validate_schema(&self, goal: &GoalDefinition, result: &mut ValidationResult) {
        // Name validation
        if goal.name.is_empty() {
            result.add(
                ValidationError::error("Goal name cannot be empty")
                    .with_field("name")
                    .with_suggestion(
                        "Provide a descriptive name like 'defend_position' or 'escort_player'",
                    ),
            );
        } else if goal.name.len() > 100 {
            result.add(
                ValidationError::warning("Goal name is very long (>100 chars)")
                    .with_field("name")
                    .with_suggestion("Consider a shorter, more concise name"),
            );
        }

        // Priority validation
        if let Some(priority) = goal.priority {
            if priority < 0.0 {
                result.add(
                    ValidationError::error(format!(
                        "Priority must be non-negative, got {}",
                        priority
                    ))
                    .with_field("priority"),
                );
            } else if priority > 10.0 {
                result.add(
                    ValidationError::warning(format!(
                        "Priority {} is very high (>10), typically 1-10 is recommended",
                        priority
                    ))
                    .with_field("priority"),
                );
            } else if priority == 0.0 {
                result.add(
                    ValidationError::warning("Priority is 0, goal will have lowest urgency")
                        .with_field("priority"),
                );
            }
        }

        // Deadline validation
        if let Some(deadline) = goal.deadline_seconds {
            if deadline < 0.0 {
                result.add(
                    ValidationError::error(format!(
                        "Deadline must be non-negative, got {}",
                        deadline
                    ))
                    .with_field("deadline_seconds"),
                );
            } else if deadline < 1.0 {
                result.add(
                    ValidationError::warning("Deadline is very short (<1s), may be unachievable")
                        .with_field("deadline_seconds"),
                );
            } else if deadline > 3600.0 {
                result.add(
                    ValidationError::info(
                        "Deadline is very long (>1 hour), urgency will be minimal",
                    )
                    .with_field("deadline_seconds"),
                );
            }
        }

        // Decomposition strategy validation
        if let Some(ref decomp) = goal.decomposition {
            match decomp.as_str() {
                "sequential" | "parallel" | "any_of" | "all_of" => {}
                other => {
                    result.add(
                        ValidationError::error(format!("Invalid decomposition strategy '{}', must be one of: sequential, parallel, any_of, all_of", other))
                            .with_field("decomposition")
                    );
                }
            }
        }

        // Max depth validation
        if let Some(max_depth) = goal.max_depth {
            if max_depth == 0 {
                result.add(
                    ValidationError::warning("Max depth is 0, sub-goals will never be decomposed")
                        .with_field("max_depth"),
                );
            } else if max_depth > 10 {
                result.add(
                    ValidationError::warning(format!(
                        "Max depth {} is very large, may cause performance issues",
                        max_depth
                    ))
                    .with_field("max_depth")
                    .with_suggestion("Recommended max depth is 3-5"),
                );
            }
        }

        // Desired state validation
        if goal.desired_state.is_empty() {
            result.add(
                ValidationError::error("Goal has no desired state conditions")
                    .with_field("desired_state")
                    .with_suggestion("Add at least one condition like 'objective_complete = true'"),
            );
        }

        // State value validation
        for (key, value) in &goal.desired_state {
            self.validate_state_value(key, value, result);
        }

        // Recursively validate sub-goals
        if let Some(ref sub_goals) = goal.sub_goals {
            for (i, sub_goal) in sub_goals.iter().enumerate() {
                let mut sub_result = ValidationResult::new();
                self.validate_schema(sub_goal, &mut sub_result);

                // Prefix sub-goal errors with path
                for mut error in sub_result.errors {
                    if let Some(field) = error.field {
                        error.field = Some(format!("sub_goals[{}].{}", i, field));
                    }
                    result.add(error);
                }
                for mut warning in sub_result.warnings {
                    if let Some(field) = warning.field {
                        warning.field = Some(format!("sub_goals[{}].{}", i, field));
                    }
                    result.add(warning);
                }
            }
        }
    }

    /// Validate state value types and ranges
    fn validate_state_value(
        &self,
        key: &str,
        value: &StateValueDef,
        result: &mut ValidationResult,
    ) {
        // Check if state variable is known
        if !self.known_state_variables.contains(key) && !key.starts_with("custom_") {
            let msg = format!("Unknown state variable '{}', may not work at runtime", key);
            if self.strict_mode {
                result.add(ValidationError::error(msg).with_field(key).with_suggestion(
                    "Add 'custom_' prefix for custom variables or verify spelling",
                ));
            } else {
                result.add(ValidationError::warning(msg).with_field(key));
            }
        }

        // Validate ranges
        match value {
            StateValueDef::IntRange { min, max } => {
                if min > max {
                    result.add(
                        ValidationError::error(format!("IntRange min ({}) > max ({})", min, max))
                            .with_field(key),
                    );
                } else if min == max {
                    result.add(
                        ValidationError::info(format!(
                            "IntRange min == max ({}), consider using exact value",
                            min
                        ))
                        .with_field(key),
                    );
                }
            }
            StateValueDef::FloatApprox {
                value: _,
                tolerance,
            } => {
                if *tolerance < 0.0 {
                    result.add(
                        ValidationError::error("FloatApprox tolerance must be non-negative")
                            .with_field(key),
                    );
                } else if *tolerance > 100.0 {
                    result.add(
                        ValidationError::warning(format!(
                            "FloatApprox tolerance {} is very large",
                            tolerance
                        ))
                        .with_field(key),
                    );
                }
            }
            _ => {}
        }
    }

    /// Semantic validation: check achievability and conflicts
    fn validate_semantics(&self, goal: &GoalDefinition, result: &mut ValidationResult) {
        // Check for sub-goals without decomposition strategy
        if let Some(ref sub_goals) = goal.sub_goals {
            if !sub_goals.is_empty() && goal.decomposition.is_none() {
                result.add(
                    ValidationError::warning(
                        "Goal has sub-goals but no decomposition strategy specified",
                    )
                    .with_field("decomposition")
                    .with_suggestion("Add 'decomposition = \"sequential\"' or other strategy"),
                );
            }

            // Check for conflicting desired states in sub-goals
            if goal.decomposition.as_deref() == Some("parallel")
                || goal.decomposition.as_deref() == Some("all_of")
            {
                self.check_conflicting_sub_goals(sub_goals, result);
            }

            // Check if any-of has at least 2 options
            if goal.decomposition.as_deref() == Some("any_of") && sub_goals.len() < 2 {
                result.add(
                    ValidationError::warning(
                        "'any_of' strategy with only one sub-goal, consider using direct goal",
                    )
                    .with_field("decomposition"),
                );
            }
        }

        // Check for decomposition without sub-goals
        if goal.decomposition.is_some() && goal.sub_goals.as_ref().map_or(true, |sg| sg.is_empty())
        {
            result.add(
                ValidationError::warning(
                    "Decomposition strategy specified but no sub-goals defined",
                )
                .with_field("decomposition"),
            );
        }
    }

    /// Check for circular dependencies in goal hierarchy
    fn check_circular_dependencies(&self, goal: &GoalDefinition, result: &mut ValidationResult) {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        self.check_circular_recursive(goal, &mut visited, &mut path, result);
    }

    fn check_circular_recursive(
        &self,
        goal: &GoalDefinition,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
        result: &mut ValidationResult,
    ) {
        if path.contains(&goal.name) {
            result.add(
                ValidationError::error(format!(
                    "Circular dependency detected: {} appears in path: {}",
                    goal.name,
                    path.join(" → ")
                ))
                .with_field(&goal.name),
            );
            return;
        }

        if visited.contains(&goal.name) {
            return; // Already checked this branch
        }

        visited.insert(goal.name.clone());
        path.push(goal.name.clone());

        if let Some(ref sub_goals) = goal.sub_goals {
            for sub_goal in sub_goals {
                self.check_circular_recursive(sub_goal, visited, path, result);
            }
        }

        path.pop();
    }

    /// Check for conflicting desired states between sub-goals
    fn check_conflicting_sub_goals(
        &self,
        sub_goals: &[GoalDefinition],
        result: &mut ValidationResult,
    ) {
        let mut state_map: HashMap<String, Vec<(usize, &StateValueDef)>> = HashMap::new();

        for (i, sub_goal) in sub_goals.iter().enumerate() {
            for (key, value) in &sub_goal.desired_state {
                state_map.entry(key.clone()).or_default().push((i, value));
            }
        }

        for (key, values) in state_map {
            if values.len() > 1 {
                // Check if values conflict
                let first_value = values[0].1;
                for (_idx, value) in &values[1..] {
                    if !self.values_compatible(first_value, value) {
                        result.add(
                            ValidationError::warning(format!(
                                "Sub-goals may have conflicting desired states for '{}': {:?} vs {:?}",
                                key, first_value, value
                            ))
                            .with_suggestion("Ensure sub-goals don't contradict each other in parallel execution")
                        );
                    }
                }
            }
        }
    }

    /// Check if two state values are compatible
    fn values_compatible(&self, v1: &StateValueDef, v2: &StateValueDef) -> bool {
        match (v1, v2) {
            (StateValueDef::Bool(b1), StateValueDef::Bool(b2)) => b1 == b2,
            (StateValueDef::Int(i1), StateValueDef::Int(i2)) => i1 == i2,
            (StateValueDef::String(s1), StateValueDef::String(s2)) => s1 == s2,
            // Ranges might overlap
            (
                StateValueDef::IntRange {
                    min: min1,
                    max: max1,
                },
                StateValueDef::IntRange {
                    min: min2,
                    max: max2,
                },
            ) => {
                !(max1 < min2 || max2 < min1) // Ranges overlap if not disjoint
            }
            _ => true, // Different types or floats, assume compatible
        }
    }

    /// Analyze goal complexity
    fn analyze_complexity(&self, goal: &GoalDefinition, result: &mut ValidationResult) {
        let depth = self.calculate_depth(goal);
        let total_goals = self.count_total_goals(goal);

        if depth > 5 {
            result.add(
                ValidationError::warning(format!(
                    "Goal hierarchy is {} levels deep, may impact performance",
                    depth
                ))
                .with_suggestion("Consider flattening or splitting into separate top-level goals"),
            );
        }

        if total_goals > 20 {
            result.add(
                ValidationError::warning(format!(
                    "Goal hierarchy contains {} total goals, very complex",
                    total_goals
                ))
                .with_suggestion("Consider simplifying or splitting into multiple goals"),
            );
        }

        // Check if all sub-goals at one level have same priority
        if let Some(ref sub_goals) = goal.sub_goals {
            if sub_goals.len() > 1 {
                let priorities: Vec<_> = sub_goals.iter().filter_map(|g| g.priority).collect();
                if priorities.len() == sub_goals.len()
                    && priorities.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01)
                {
                    result.add(
                        ValidationError::info(
                            "All sub-goals have same priority, order may be arbitrary",
                        )
                        .with_suggestion("Differentiate priorities if order matters"),
                    );
                }
            }
        }
    }

    fn calculate_depth(&self, goal: &GoalDefinition) -> usize {
        if let Some(ref sub_goals) = goal.sub_goals {
            if sub_goals.is_empty() {
                1
            } else {
                1 + sub_goals
                    .iter()
                    .map(|g| self.calculate_depth(g))
                    .max()
                    .unwrap_or(0)
            }
        } else {
            1
        }
    }

    fn count_total_goals(&self, goal: &GoalDefinition) -> usize {
        1 + if let Some(ref sub_goals) = goal.sub_goals {
            sub_goals.iter().map(|g| self.count_total_goals(g)).sum()
        } else {
            0
        }
    }

    /// Get default known state variables
    fn default_state_variables() -> HashSet<String> {
        vec![
            // Player state
            "player_hp",
            "player_x",
            "player_y",
            "player_is_wounded",
            "player_is_critical",
            "player_is_down",
            "player_at_extraction",
            "player_alive",
            // Companion state
            "my_ammo",
            "my_x",
            "my_y",
            "my_morale",
            "my_is_wounded",
            "my_is_critical",
            "my_has_ammo",
            "my_needs_reload",
            "my_hp",
            // Combat state
            "enemy_count",
            "in_combat",
            "enemy_hp",
            "enemy_x",
            "enemy_y",
            "enemy_is_wounded",
            "enemy_is_critical",
            "enemy_distance",
            "enemy_in_range_melee",
            "enemy_in_range_short",
            "enemy_in_range_long",
            "enemy_defeated",
            "enemy_damaged",
            "threats_neutralized",
            "threats_in_path",
            // Tactical state
            "needs_healing",
            "can_move",
            "has_cover_available",
            "in_cover",
            "weapon_equipped",
            "has_weapon",
            "in_range",
            "at_location",
            "area_scanned",
            "enemies_located",
            "path_clear",
            "position_held",
            "enemies_suppressed",
            "reinforcements_called",
            "ready_for_combat",
            // Mission state
            "at_extraction",
            "mission_complete",
            "objective_complete",
            "position_captured",
            "enemies_defeated",
            "position_secured",
            // Revive state
            "ally_revived",
            "ally_safe",
            "safe_zone_established",
            "smoke_deployed",
            "visibility_reduced",
            "distance_to_ally",
            "ally_health",
            "ally_conscious",
            "nearby_enemies",
            // Patrol state
            "area_patrolled",
            "threats_reported",
            "at_waypoint_1",
            "waypoint_1_scanned",
            "at_waypoint_2",
            "waypoint_2_scanned",
            "at_waypoint_3",
            "waypoint_3_scanned",
            "at_patrol_start",
            // Equipment state
            "ammo_loaded",
            "grenades_ready",
            "team_coordinated",
            "equipment_ready",
            "cover_quality",
            "fortified",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }
}

impl Default for GoalValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn create_simple_goal() -> GoalDefinition {
        let mut desired = BTreeMap::new();
        desired.insert("enemy_defeated".to_string(), StateValueDef::Bool(true));

        GoalDefinition {
            name: "defeat_enemy".to_string(),
            priority: Some(5.0),
            deadline_seconds: Some(60.0),
            decomposition: None,
            max_depth: Some(5),
            desired_state: desired,
            sub_goals: None,
        }
    }

    #[test]
    fn test_valid_goal() {
        let validator = GoalValidator::new();
        let goal = create_simple_goal();
        let result = validator.validate(&goal);

        assert!(result.is_valid(), "Expected no errors: {:?}", result.errors);
    }

    #[test]
    fn test_empty_name() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.name = "".to_string();

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("name cannot be empty")));
    }

    #[test]
    fn test_negative_priority() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.priority = Some(-1.0);

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("non-negative")));
    }

    #[test]
    fn test_invalid_decomposition() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("invalid_strategy".to_string());

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Invalid decomposition")));
    }

    #[test]
    fn test_circular_dependency() {
        let validator = GoalValidator::new();

        let mut desired = BTreeMap::new();
        desired.insert("flag".to_string(), StateValueDef::Bool(true));

        // Create circular: A → B → A
        let goal_a_inner = GoalDefinition {
            name: "goal_a".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: desired.clone(),
            sub_goals: None,
        };

        let goal_b = GoalDefinition {
            name: "goal_b".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: Some("sequential".to_string()),
            max_depth: None,
            desired_state: desired.clone(),
            sub_goals: Some(vec![goal_a_inner]),
        };

        let goal_a = GoalDefinition {
            name: "goal_a".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: Some("sequential".to_string()),
            max_depth: None,
            desired_state: desired,
            sub_goals: Some(vec![goal_b]),
        };

        let result = validator.validate(&goal_a);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Circular dependency")));
    }

    #[test]
    fn test_unknown_state_variable_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "completely_unknown_var".to_string(),
            StateValueDef::Bool(true),
        );

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("Unknown state variable")));
    }

    #[test]
    fn test_complexity_warnings() {
        let validator = GoalValidator::new();

        // Create very deep hierarchy
        let mut goal = create_simple_goal();
        let mut current = &mut goal;

        for i in 0..7 {
            let mut sub_desired = BTreeMap::new();
            sub_desired.insert(format!("level_{}", i), StateValueDef::Bool(true));

            let sub_goal = GoalDefinition {
                name: format!("level_{}", i),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: Some("sequential".to_string()),
                max_depth: None,
                desired_state: sub_desired,
                sub_goals: None,
            };

            current.sub_goals = Some(vec![sub_goal]);
            current = current.sub_goals.as_mut().unwrap().get_mut(0).unwrap();
        }

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("levels deep")));
    }

    #[test]
    fn test_invalid_int_range() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "health".to_string(),
            StateValueDef::IntRange { min: 100, max: 50 }, // Invalid: min > max
        );

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("IntRange min")));
    }

    // ========== Mutation-killing tests ==========

    #[test]
    fn test_total_issues() {
        let mut result = ValidationResult::new();
        assert_eq!(result.total_issues(), 0);

        result.add(ValidationError::error("e1"));
        assert_eq!(result.total_issues(), 1);

        result.add(ValidationError::warning("w1"));
        assert_eq!(result.total_issues(), 2);

        result.add(ValidationError::info("i1"));
        assert_eq!(result.total_issues(), 3);

        result.add(ValidationError::error("e2"));
        assert_eq!(result.total_issues(), 4);
    }

    #[test]
    fn test_merge() {
        let mut result1 = ValidationResult::new();
        result1.add(ValidationError::error("e1"));
        result1.add(ValidationError::warning("w1"));

        let mut result2 = ValidationResult::new();
        result2.add(ValidationError::error("e2"));
        result2.add(ValidationError::info("i1"));

        result1.merge(result2);

        assert_eq!(result1.errors.len(), 2);
        assert_eq!(result1.warnings.len(), 1);
        assert_eq!(result1.info.len(), 1);
        assert!(result1.errors.iter().any(|e| e.message == "e2"));
    }

    #[test]
    fn test_with_strict_mode() {
        let validator = GoalValidator::new().with_strict_mode(true);
        let mut goal = create_simple_goal();

        // Unknown variable in strict mode → error, not warning
        goal.desired_state.insert(
            "completely_unknown_var".to_string(),
            StateValueDef::Bool(true),
        );

        let result = validator.validate(&goal);
        assert!(!result.is_valid()); // Has errors, not just warnings
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("Unknown state variable")));
    }

    #[test]
    fn test_strict_mode_false_gives_warning() {
        let validator = GoalValidator::new().with_strict_mode(false);
        let mut goal = create_simple_goal();

        goal.desired_state
            .insert("my_made_up_var".to_string(), StateValueDef::Bool(true));

        let result = validator.validate(&goal);
        // Should be valid (only warnings)
        assert!(result.is_valid());
        assert!(result.has_warnings());
    }

    #[test]
    fn test_add_known_state_variable() {
        let mut validator = GoalValidator::new();
        validator.add_known_state_variable("my_custom_var");

        let mut goal = create_simple_goal();
        goal.desired_state
            .insert("my_custom_var".to_string(), StateValueDef::Bool(true));

        let result = validator.validate(&goal);
        // No unknown variable warnings
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("my_custom_var")));
    }

    #[test]
    fn test_default() {
        let validator = GoalValidator::default();
        let goal = create_simple_goal();
        let result = validator.validate(&goal);
        assert!(result.is_valid()); // Default should work like new()
    }

    #[test]
    fn test_long_name_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.name = "a".repeat(101); // > 100 chars

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("very long")));
    }

    #[test]
    fn test_name_exactly_100_chars() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.name = "a".repeat(100); // exactly 100 → no warning

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("very long")));
    }

    #[test]
    fn test_priority_zero_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.priority = Some(0.0);

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("lowest urgency")));
    }

    #[test]
    fn test_priority_high_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.priority = Some(11.0); // > 10.0

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("very high")));
    }

    #[test]
    fn test_priority_10_no_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.priority = Some(10.0); // exactly 10 → no "very high" warning

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("very high")));
    }

    #[test]
    fn test_deadline_short_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.deadline_seconds = Some(0.5); // < 1.0

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("very short")));
    }

    #[test]
    fn test_deadline_1_no_short_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.deadline_seconds = Some(1.0);

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("very short")));
    }

    #[test]
    fn test_deadline_long_info() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.deadline_seconds = Some(3601.0); // > 3600.0

        let result = validator.validate(&goal);
        assert!(result.info.iter().any(|i| i.message.contains("very long")));
    }

    #[test]
    fn test_deadline_3600_no_info() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.deadline_seconds = Some(3600.0); // exactly 3600 → no info

        let result = validator.validate(&goal);
        assert!(!result.info.iter().any(|i| i.message.contains("very long")));
    }

    #[test]
    fn test_negative_deadline_error() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.deadline_seconds = Some(-1.0);

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("non-negative")));
    }

    #[test]
    fn test_max_depth_zero_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.max_depth = Some(0);

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("Max depth is 0")));
    }

    #[test]
    fn test_max_depth_large_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.max_depth = Some(11); // > 10

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("very large")));
    }

    #[test]
    fn test_max_depth_10_no_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.max_depth = Some(10); // exactly 10 → no warning

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("very large")));
    }

    #[test]
    fn test_int_range_min_equals_max_info() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "enemy_count".to_string(),
            StateValueDef::IntRange { min: 5, max: 5 },
        );

        let result = validator.validate(&goal);
        assert!(result.info.iter().any(|i| i.message.contains("min == max")));
    }

    #[test]
    fn test_float_approx_negative_tolerance() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "player_hp".to_string(),
            StateValueDef::FloatApprox {
                value: 50.0,
                tolerance: -1.0,
            },
        );

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("tolerance must be non-negative")));
    }

    #[test]
    fn test_float_approx_large_tolerance() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "player_hp".to_string(),
            StateValueDef::FloatApprox {
                value: 50.0,
                tolerance: 150.0, // > 100.0
            },
        );

        let result = validator.validate(&goal);
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("very large")));
    }

    #[test]
    fn test_float_approx_100_no_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.desired_state.insert(
            "player_hp".to_string(),
            StateValueDef::FloatApprox {
                value: 50.0,
                tolerance: 100.0, // exactly 100 → no warning
            },
        );

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("tolerance") && w.message.contains("very large")));
    }

    #[test]
    fn test_sub_goals_without_decomposition() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();

        let mut sub_desired = BTreeMap::new();
        sub_desired.insert("enemy_defeated".to_string(), StateValueDef::Bool(true));

        goal.sub_goals = Some(vec![GoalDefinition {
            name: "sub1".to_string(),
            priority: Some(3.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: sub_desired,
            sub_goals: None,
        }]);
        // decomposition is None but sub_goals exist

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("no decomposition strategy")));
    }

    #[test]
    fn test_decomposition_without_sub_goals() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("sequential".to_string());
        // No sub_goals

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("no sub-goals defined")));
    }

    #[test]
    fn test_any_of_with_one_sub_goal() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("any_of".to_string());

        let mut sub_desired = BTreeMap::new();
        sub_desired.insert("enemy_defeated".to_string(), StateValueDef::Bool(true));

        goal.sub_goals = Some(vec![GoalDefinition {
            name: "only_option".to_string(),
            priority: Some(3.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: sub_desired,
            sub_goals: None,
        }]);

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("only one sub-goal")));
    }

    #[test]
    fn test_conflicting_sub_goals_parallel() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("parallel".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert("in_combat".to_string(), StateValueDef::Bool(true));

        let mut desired2 = BTreeMap::new();
        desired2.insert("in_combat".to_string(), StateValueDef::Bool(false));

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "attack".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "retreat".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_conflicting_sub_goals_all_of() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("all_of".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert("enemy_count".to_string(), StateValueDef::Int(0));

        let mut desired2 = BTreeMap::new();
        desired2.insert("enemy_count".to_string(), StateValueDef::Int(5));

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "clear_area".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "gather_enemies".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_compatible_sub_goals_no_conflict() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("parallel".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert("in_combat".to_string(), StateValueDef::Bool(true));

        let mut desired2 = BTreeMap::new();
        desired2.insert("in_combat".to_string(), StateValueDef::Bool(true));

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "attack1".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "attack2".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_conflicting_int_ranges_disjoint() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("parallel".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert(
            "enemy_count".to_string(),
            StateValueDef::IntRange { min: 0, max: 3 },
        );

        let mut desired2 = BTreeMap::new();
        desired2.insert(
            "enemy_count".to_string(),
            StateValueDef::IntRange { min: 5, max: 10 },
        );

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "few".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "many".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        // Disjoint ranges → conflict warning
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_overlapping_int_ranges_compatible() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("parallel".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert(
            "enemy_count".to_string(),
            StateValueDef::IntRange { min: 0, max: 5 },
        );

        let mut desired2 = BTreeMap::new();
        desired2.insert(
            "enemy_count".to_string(),
            StateValueDef::IntRange { min: 3, max: 10 },
        );

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "range1".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "range2".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        // Overlapping ranges → NO conflict
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_conflicting_strings() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("all_of".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert(
            "custom_state".to_string(),
            StateValueDef::String("attack".to_string()),
        );

        let mut desired2 = BTreeMap::new();
        desired2.insert(
            "custom_state".to_string(),
            StateValueDef::String("retreat".to_string()),
        );

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "offense".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "defense".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("conflicting")));
    }

    #[test]
    fn test_total_goals_over_20_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("sequential".to_string());

        // Create 21 sub-goals (total = 22 with root)
        let mut sub_goals = Vec::new();
        for i in 0..21 {
            let mut desired = BTreeMap::new();
            desired.insert(format!("flag_{}", i), StateValueDef::Bool(true));

            sub_goals.push(GoalDefinition {
                name: format!("sub_{}", i),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired,
                sub_goals: None,
            });
        }
        goal.sub_goals = Some(sub_goals);

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("total goals")));
    }

    #[test]
    fn test_same_priority_info() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("sequential".to_string());

        let mut desired1 = BTreeMap::new();
        desired1.insert("enemy_defeated".to_string(), StateValueDef::Bool(true));

        let mut desired2 = BTreeMap::new();
        desired2.insert("in_cover".to_string(), StateValueDef::Bool(true));

        goal.sub_goals = Some(vec![
            GoalDefinition {
                name: "sub1".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired1,
                sub_goals: None,
            },
            GoalDefinition {
                name: "sub2".to_string(),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: None,
                max_depth: None,
                desired_state: desired2,
                sub_goals: None,
            },
        ]);

        let result = validator.validate(&goal);
        assert!(
            result
                .info
                .iter()
                .any(|i| i.message.contains("same priority")),
            "Expected same priority info: {:?}",
            result.info
        );
    }

    #[test]
    fn test_empty_desired_state_error() {
        let validator = GoalValidator::new();

        let goal = GoalDefinition {
            name: "empty_goal".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: BTreeMap::new(),
            sub_goals: None,
        };

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result
            .errors
            .iter()
            .any(|e| e.message.contains("no desired state")));
    }

    #[test]
    fn test_custom_prefix_no_warning() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();

        // custom_ prefix should not trigger unknown variable warning
        goal.desired_state.insert(
            "custom_my_special_flag".to_string(),
            StateValueDef::Bool(true),
        );

        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("custom_my_special_flag")));
    }

    #[test]
    fn test_validation_error_with_field() {
        let err = ValidationError::error("test").with_field("my_field");
        assert_eq!(err.field, Some("my_field".to_string()));
    }

    #[test]
    fn test_validation_error_with_suggestion() {
        let err = ValidationError::error("test").with_suggestion("try this");
        assert_eq!(err.suggestion, Some("try this".to_string()));
    }

    #[test]
    fn test_validation_result_routing() {
        let mut result = ValidationResult::new();

        result.add(ValidationError::error("err"));
        result.add(ValidationError::warning("warn"));
        result.add(ValidationError::info("info"));

        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.info.len(), 1);

        assert_eq!(result.errors[0].message, "err");
        assert_eq!(result.warnings[0].message, "warn");
        assert_eq!(result.info[0].message, "info");
    }

    #[test]
    fn test_is_valid_and_has_warnings() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert!(!result.has_warnings());

        result.add(ValidationError::warning("w"));
        assert!(result.is_valid()); // warnings don't invalidate
        assert!(result.has_warnings());

        result.add(ValidationError::error("e"));
        assert!(!result.is_valid());
    }

    #[test]
    fn test_sub_goal_schema_errors_prefixed() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("sequential".to_string());

        // Sub-goal with empty name → error with prefixed field
        let sub_goal = GoalDefinition {
            name: "".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: BTreeMap::from([(
                "enemy_defeated".to_string(),
                StateValueDef::Bool(true),
            )]),
            sub_goals: None,
        };

        goal.sub_goals = Some(vec![sub_goal]);

        let result = validator.validate(&goal);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| {
            e.field
                .as_ref()
                .map_or(false, |f| f.starts_with("sub_goals[0]"))
        }));
    }

    #[test]
    fn test_calculate_depth_flat() {
        let validator = GoalValidator::new();
        let goal = create_simple_goal();
        // No sub-goals → depth 1
        // Indirectly verified via complexity (depth > 5 triggers warning)
        let result = validator.validate(&goal);
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("levels deep")));
    }

    #[test]
    fn test_calculate_depth_exactly_5() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        let mut current = &mut goal;

        // Create exactly 5 levels deep (1 root + 4 sub-goals = depth 5)
        for i in 0..4 {
            let mut sub_desired = BTreeMap::new();
            sub_desired.insert(format!("level_{}", i), StateValueDef::Bool(true));

            let sub_goal = GoalDefinition {
                name: format!("level_{}", i),
                priority: Some(5.0),
                deadline_seconds: None,
                decomposition: Some("sequential".to_string()),
                max_depth: None,
                desired_state: sub_desired,
                sub_goals: None,
            };

            current.sub_goals = Some(vec![sub_goal]);
            current = current.sub_goals.as_mut().unwrap().get_mut(0).unwrap();
        }

        let result = validator.validate(&goal);
        // Depth = 5 → no warning (needs > 5)
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.message.contains("levels deep")));
    }

    #[test]
    fn test_decomposition_with_empty_sub_goals() {
        let validator = GoalValidator::new();
        let mut goal = create_simple_goal();
        goal.decomposition = Some("sequential".to_string());
        goal.sub_goals = Some(vec![]); // Empty vec, not None

        let result = validator.validate(&goal);
        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("no sub-goals defined")));
    }

    #[test]
    fn test_valid_decomposition_strategies() {
        let validator = GoalValidator::new();

        for strategy in &["sequential", "parallel", "any_of", "all_of"] {
            let mut goal = create_simple_goal();
            goal.decomposition = Some(strategy.to_string());

            let mut sub_desired = BTreeMap::new();
            sub_desired.insert("enemy_defeated".to_string(), StateValueDef::Bool(true));

            goal.sub_goals = Some(vec![
                GoalDefinition {
                    name: "sub1".to_string(),
                    priority: Some(3.0),
                    deadline_seconds: None,
                    decomposition: None,
                    max_depth: None,
                    desired_state: sub_desired.clone(),
                    sub_goals: None,
                },
                GoalDefinition {
                    name: "sub2".to_string(),
                    priority: Some(4.0),
                    deadline_seconds: None,
                    decomposition: None,
                    max_depth: None,
                    desired_state: sub_desired.clone(),
                    sub_goals: None,
                },
            ]);

            let result = validator.validate(&goal);
            assert!(
                !result
                    .errors
                    .iter()
                    .any(|e| e.message.contains("Invalid decomposition")),
                "Strategy '{}' should be valid",
                strategy
            );
        }
    }

    #[test]
    fn test_strict_mode_with_custom_prefix() {
        // Even in strict mode, custom_ prefix should not error
        let validator = GoalValidator::new().with_strict_mode(true);
        let mut goal = create_simple_goal();
        goal.desired_state
            .insert("custom_flag".to_string(), StateValueDef::Bool(true));

        let result = validator.validate(&goal);
        assert!(!result
            .errors
            .iter()
            .any(|e| e.message.contains("custom_flag")));
    }
}
