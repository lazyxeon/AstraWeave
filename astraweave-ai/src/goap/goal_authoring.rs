use super::{DecompositionStrategy, Goal, StateValue};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// TOML-friendly goal definition for designer authoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDefinition {
    pub name: String,
    pub priority: Option<f32>,
    pub deadline_seconds: Option<f32>,
    pub decomposition: Option<String>, // "sequential", "parallel", "any_of", "all_of"
    pub max_depth: Option<usize>,
    pub desired_state: BTreeMap<String, StateValueDef>,
    pub sub_goals: Option<Vec<GoalDefinition>>,
}

/// TOML-friendly state value definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum StateValueDef {
    Bool(bool),
    Int(i32),
    Float(f64),
    String(String),
    IntRange { min: i32, max: i32 },
    FloatApprox { value: f64, tolerance: f64 },
}

impl StateValueDef {
    /// Convert to internal StateValue
    pub fn to_state_value(&self) -> StateValue {
        match self {
            StateValueDef::Bool(b) => StateValue::Bool(*b),
            StateValueDef::Int(i) => StateValue::Int(*i),
            StateValueDef::Float(f) => StateValue::Float(super::OrderedFloat(*f as f32)),
            StateValueDef::String(s) => StateValue::String(s.clone()),
            StateValueDef::IntRange { min, max } => StateValue::IntRange(*min, *max),
            StateValueDef::FloatApprox { value, tolerance } => {
                StateValue::FloatApprox(*value as f32, *tolerance as f32)
            }
        }
    }
}

impl GoalDefinition {
    /// Load goal definition from TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read goal file {}: {}", path.display(), e))?;

        let goal: GoalDefinition = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse TOML in {}: {}", path.display(), e))?;

        goal.validate()?;
        Ok(goal)
    }

    /// Save goal definition to TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;

        let content =
            toml::to_string_pretty(self).map_err(|e| anyhow!("Failed to serialize goal: {}", e))?;

        std::fs::write(path, content)
            .map_err(|e| anyhow!("Failed to write goal file {}: {}", path.display(), e))?;

        Ok(())
    }

    /// Validate goal definition
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(anyhow!("Goal name cannot be empty"));
        }

        if let Some(priority) = self.priority {
            if priority < 0.0 {
                return Err(anyhow!(
                    "Goal priority must be non-negative, got {}",
                    priority
                ));
            }
        }

        if let Some(deadline) = self.deadline_seconds {
            if deadline < 0.0 {
                return Err(anyhow!(
                    "Goal deadline must be non-negative, got {}",
                    deadline
                ));
            }
        }

        if let Some(ref decomp) = self.decomposition {
            match decomp.as_str() {
                "sequential" | "parallel" | "any_of" | "all_of" => {}
                other => return Err(anyhow!("Invalid decomposition strategy: {}", other)),
            }
        }

        // Validate sub-goals recursively
        if let Some(ref sub_goals) = self.sub_goals {
            for sub_goal in sub_goals {
                sub_goal.validate()?;
            }
        }

        Ok(())
    }

    /// Convert to internal Goal representation
    pub fn to_goal(&self) -> Goal {
        let mut desired_state = BTreeMap::new();
        for (key, value_def) in &self.desired_state {
            desired_state.insert(key.clone(), value_def.to_state_value());
        }

        let mut goal = Goal::new(&self.name, desired_state);

        if let Some(priority) = self.priority {
            goal = goal.with_priority(priority);
        }

        if let Some(deadline) = self.deadline_seconds {
            goal = goal.with_deadline(deadline);
        }

        if let Some(max_depth) = self.max_depth {
            goal = goal.with_max_depth(max_depth);
        }

        if let Some(ref decomp_str) = self.decomposition {
            let strategy = match decomp_str.as_str() {
                "sequential" => DecompositionStrategy::Sequential,
                "parallel" => DecompositionStrategy::Parallel,
                "any_of" => DecompositionStrategy::AnyOf,
                "all_of" => DecompositionStrategy::AllOf,
                _ => DecompositionStrategy::Sequential, // Default fallback
            };
            goal = goal.with_strategy(strategy);
        }

        if let Some(ref sub_goal_defs) = self.sub_goals {
            let sub_goals: Vec<Goal> = sub_goal_defs.iter().map(|def| def.to_goal()).collect();
            goal = goal.with_sub_goals(sub_goals);
        }

        goal
    }

    /// Create from internal Goal representation
    pub fn from_goal(goal: &Goal) -> Self {
        let desired_state: BTreeMap<String, StateValueDef> = goal
            .desired_state
            .iter()
            .map(|(k, v)| {
                let value_def = match v {
                    StateValue::Bool(b) => StateValueDef::Bool(*b),
                    StateValue::Int(i) => StateValueDef::Int(*i),
                    StateValue::Float(f) => StateValueDef::Float(f.0 as f64),
                    StateValue::String(s) => StateValueDef::String(s.clone()),
                    StateValue::IntRange(min, max) => StateValueDef::IntRange {
                        min: *min,
                        max: *max,
                    },
                    StateValue::FloatApprox(f, tol) => StateValueDef::FloatApprox {
                        value: *f as f64,
                        tolerance: *tol as f64,
                    },
                };
                (k.clone(), value_def)
            })
            .collect();

        let decomposition = Some(match goal.decomposition_strategy {
            DecompositionStrategy::Sequential => "sequential".to_string(),
            DecompositionStrategy::Parallel => "parallel".to_string(),
            DecompositionStrategy::AnyOf => "any_of".to_string(),
            DecompositionStrategy::AllOf => "all_of".to_string(),
        });

        let sub_goals = if goal.sub_goals.is_empty() {
            None
        } else {
            Some(
                goal.sub_goals
                    .iter()
                    .map(GoalDefinition::from_goal)
                    .collect(),
            )
        };

        Self {
            name: goal.name.clone(),
            priority: Some(goal.priority),
            deadline_seconds: goal.deadline,
            decomposition,
            max_depth: Some(goal.max_depth),
            desired_state,
            sub_goals,
        }
    }
}

/// Goal library for loading multiple goal templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalLibrary {
    pub goals: Vec<GoalDefinition>,
}

impl GoalLibrary {
    /// Load goal library from TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read goal library {}: {}", path.display(), e))?;

        let library: GoalLibrary = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse TOML in {}: {}", path.display(), e))?;

        // Validate all goals
        for goal in &library.goals {
            goal.validate()?;
        }

        Ok(library)
    }

    /// Save goal library to TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize goal library: {}", e))?;

        std::fs::write(path, content)
            .map_err(|e| anyhow!("Failed to write goal library {}: {}", path.display(), e))?;

        Ok(())
    }

    /// Get a goal by name
    pub fn get_goal(&self, name: &str) -> Option<&GoalDefinition> {
        self.goals.iter().find(|g| g.name == name)
    }

    /// Convert all goals to internal representation
    pub fn to_goals(&self) -> Vec<Goal> {
        self.goals.iter().map(|def| def.to_goal()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_goal_definition_validation() {
        let mut goal = GoalDefinition {
            name: "test_goal".to_string(),
            priority: Some(5.0),
            deadline_seconds: Some(10.0),
            decomposition: Some("sequential".to_string()),
            max_depth: Some(3),
            desired_state: BTreeMap::new(),
            sub_goals: None,
        };

        assert!(goal.validate().is_ok());

        // Invalid priority
        goal.priority = Some(-1.0);
        assert!(goal.validate().is_err());
        goal.priority = Some(5.0);

        // Invalid decomposition
        goal.decomposition = Some("invalid".to_string());
        assert!(goal.validate().is_err());
        goal.decomposition = Some("sequential".to_string());

        // Empty name
        goal.name = "".to_string();
        assert!(goal.validate().is_err());
    }

    #[test]
    fn test_state_value_conversion() {
        let bool_val = StateValueDef::Bool(true);
        assert!(matches!(bool_val.to_state_value(), StateValue::Bool(true)));

        let int_val = StateValueDef::Int(42);
        assert!(matches!(int_val.to_state_value(), StateValue::Int(42)));

        let range_val = StateValueDef::IntRange { min: 0, max: 100 };
        assert!(matches!(
            range_val.to_state_value(),
            StateValue::IntRange(0, 100)
        ));
    }

    #[test]
    fn test_goal_conversion() {
        let mut desired_state = BTreeMap::new();
        desired_state.insert("health".to_string(), StateValueDef::Int(100));
        desired_state.insert("armed".to_string(), StateValueDef::Bool(true));

        let goal_def = GoalDefinition {
            name: "combat_ready".to_string(),
            priority: Some(8.0),
            deadline_seconds: Some(30.0),
            decomposition: Some("sequential".to_string()),
            max_depth: Some(5),
            desired_state,
            sub_goals: None,
        };

        let goal = goal_def.to_goal();
        assert_eq!(goal.name, "combat_ready");
        assert_eq!(goal.priority, 8.0);
        assert_eq!(goal.deadline, Some(30.0));
        assert_eq!(goal.max_depth, 5);
        assert_eq!(
            goal.decomposition_strategy,
            DecompositionStrategy::Sequential
        );
        assert_eq!(goal.desired_state.len(), 2);
    }

    #[test]
    fn test_goal_roundtrip() {
        let mut desired_state = BTreeMap::new();
        desired_state.insert("x".to_string(), StateValue::Int(10));
        desired_state.insert("y".to_string(), StateValue::Bool(false));

        let original_goal = Goal::new("test", desired_state)
            .with_priority(7.0)
            .with_deadline(20.0)
            .with_strategy(DecompositionStrategy::Parallel);

        let goal_def = GoalDefinition::from_goal(&original_goal);
        let converted_goal = goal_def.to_goal();

        assert_eq!(converted_goal.name, original_goal.name);
        assert_eq!(converted_goal.priority, original_goal.priority);
        assert_eq!(converted_goal.deadline, original_goal.deadline);
        assert_eq!(
            converted_goal.decomposition_strategy,
            original_goal.decomposition_strategy
        );
    }

    #[test]
    fn test_goal_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_goal.toml");

        let mut desired_state = BTreeMap::new();
        desired_state.insert("flag".to_string(), StateValueDef::Bool(true));

        let goal_def = GoalDefinition {
            name: "set_flag".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: Some("sequential".to_string()),
            max_depth: Some(5),
            desired_state,
            sub_goals: None,
        };

        goal_def.save(&file_path).unwrap();
        let loaded = GoalDefinition::load(&file_path).unwrap();

        assert_eq!(loaded.name, "set_flag");
        assert_eq!(loaded.priority, Some(5.0));
    }

    #[test]
    fn test_hierarchical_goal_definition() {
        let mut sub_desired = BTreeMap::new();
        sub_desired.insert("sub_condition".to_string(), StateValueDef::Bool(true));

        let sub_goal = GoalDefinition {
            name: "sub_goal".to_string(),
            priority: Some(3.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: sub_desired,
            sub_goals: None,
        };

        let mut main_desired = BTreeMap::new();
        main_desired.insert("main_condition".to_string(), StateValueDef::Bool(true));

        let main_goal = GoalDefinition {
            name: "main_goal".to_string(),
            priority: Some(10.0),
            deadline_seconds: Some(60.0),
            decomposition: Some("sequential".to_string()),
            max_depth: Some(3),
            desired_state: main_desired,
            sub_goals: Some(vec![sub_goal]),
        };

        assert!(main_goal.validate().is_ok());

        let goal = main_goal.to_goal();
        assert_eq!(goal.sub_goals.len(), 1);
        assert_eq!(goal.sub_goals[0].name, "sub_goal");
    }

    #[test]
    fn test_goal_library() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("goal_library.toml");

        let mut goal1_state = BTreeMap::new();
        goal1_state.insert("condition1".to_string(), StateValueDef::Bool(true));

        let goal1 = GoalDefinition {
            name: "goal1".to_string(),
            priority: Some(5.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: goal1_state,
            sub_goals: None,
        };

        let mut goal2_state = BTreeMap::new();
        goal2_state.insert("condition2".to_string(), StateValueDef::Int(100));

        let goal2 = GoalDefinition {
            name: "goal2".to_string(),
            priority: Some(8.0),
            deadline_seconds: Some(30.0),
            decomposition: None,
            max_depth: None,
            desired_state: goal2_state,
            sub_goals: None,
        };

        let library = GoalLibrary {
            goals: vec![goal1, goal2],
        };

        library.save(&file_path).unwrap();
        let loaded = GoalLibrary::load(&file_path).unwrap();

        assert_eq!(loaded.goals.len(), 2);
        assert!(loaded.get_goal("goal1").is_some());
        assert!(loaded.get_goal("goal2").is_some());
        assert!(loaded.get_goal("nonexistent").is_none());
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    /// Kills line 81: `priority < 0.0` → `<= 0.0`
    #[test]
    fn test_priority_zero_is_valid() {
        let goal = GoalDefinition {
            name: "test".to_string(),
            priority: Some(0.0),
            deadline_seconds: None,
            decomposition: None,
            max_depth: None,
            desired_state: BTreeMap::new(),
            sub_goals: None,
        };
        assert!(goal.validate().is_ok(), "priority=0.0 should be valid");
    }

    /// Kills line 90: `deadline < 0.0` → `<= 0.0` and `< → ==`
    #[test]
    fn test_deadline_boundaries() {
        let mut goal = GoalDefinition {
            name: "test".to_string(),
            priority: None,
            deadline_seconds: Some(0.0),
            decomposition: None,
            max_depth: None,
            desired_state: BTreeMap::new(),
            sub_goals: None,
        };
        assert!(goal.validate().is_ok(), "deadline=0.0 should be valid");

        goal.deadline_seconds = Some(-1.0);
        assert!(goal.validate().is_err(), "deadline=-1.0 should be invalid");
    }

    /// Kills lines 140-141: decomposition "any_of" and "all_of" match arms
    #[test]
    fn test_decomposition_any_of_and_all_of() {
        let mut goal_def = GoalDefinition {
            name: "test".to_string(),
            priority: None,
            deadline_seconds: None,
            decomposition: Some("any_of".to_string()),
            max_depth: None,
            desired_state: BTreeMap::new(),
            sub_goals: None,
        };

        let goal = goal_def.to_goal();
        assert_eq!(
            goal.decomposition_strategy,
            DecompositionStrategy::AnyOf,
            "decomposition='any_of' should produce AnyOf"
        );

        goal_def.decomposition = Some("all_of".to_string());
        let goal2 = goal_def.to_goal();
        assert_eq!(
            goal2.decomposition_strategy,
            DecompositionStrategy::AllOf,
            "decomposition='all_of' should produce AllOf"
        );
    }

    /// Kills line 250: `to_goals()` body replaced with `vec![]`
    #[test]
    fn test_goal_library_to_goals() {
        let mut state = BTreeMap::new();
        state.insert("ready".to_string(), StateValueDef::Bool(true));

        let library = GoalLibrary {
            goals: vec![
                GoalDefinition {
                    name: "g1".to_string(),
                    priority: Some(1.0),
                    deadline_seconds: None,
                    decomposition: None,
                    max_depth: None,
                    desired_state: state.clone(),
                    sub_goals: None,
                },
                GoalDefinition {
                    name: "g2".to_string(),
                    priority: Some(2.0),
                    deadline_seconds: None,
                    decomposition: None,
                    max_depth: None,
                    desired_state: state,
                    sub_goals: None,
                },
            ],
        };

        let goals = library.to_goals();
        assert_eq!(goals.len(), 2, "to_goals should return 2 goals");
        assert_eq!(goals[0].name, "g1");
        assert_eq!(goals[1].name, "g2");
    }
}
