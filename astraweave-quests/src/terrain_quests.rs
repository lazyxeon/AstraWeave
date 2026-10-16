//! Terrain-Driven Quest Generation
//!
//! This module extends the LLM quest system to generate quests based on
//! dynamically generated terrain features. When terrain is modified by the
//! AI orchestrator, appropriate quests can be generated to utilize the new
//! features.
//!
//! # Overview
//!
//! - `TerrainQuestContext` - Context for terrain-driven quest generation
//! - `TerrainQuestTrigger` - Triggers quests based on terrain changes
//! - `TerrainQuestGenerator` - Generates quests tied to terrain features

use crate::{
    LlmQuest, LlmQuestStep, ObjectiveType, QuestContext, QuestGenerationConfig, QuestMetadata,
    QuestObjective, QuestRewards, ValidationCriteria,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Types of terrain features that can trigger quests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TerrainFeatureType {
    Mountain,
    Hill,
    Valley,
    Cliff,
    Canyon,
    River,
    Lake,
    Waterfall,
    Pond,
    Stream,
    Forest,
    Grove,
    Meadow,
    Marsh,
    Desert,
    Cave,
    Crater,
    Plateau,
    Ridge,
    Ravine,
}

impl TerrainFeatureType {
    /// Get quest archetypes for this terrain feature
    pub fn quest_archetypes(&self) -> Vec<&'static str> {
        match self {
            TerrainFeatureType::Mountain => {
                vec!["climb", "explore_summit", "rescue", "retrieve_artifact"]
            }
            TerrainFeatureType::Hill => vec!["survey", "fortify", "gather_resources"],
            TerrainFeatureType::Valley => vec!["settle", "defend", "explore_ruins"],
            TerrainFeatureType::Cliff => vec!["rescue", "treasure_hunt", "navigation"],
            TerrainFeatureType::Canyon => vec!["traverse", "discover_secrets", "ambush"],
            TerrainFeatureType::River => vec!["cross", "fish", "follow_downstream", "build_bridge"],
            TerrainFeatureType::Lake => vec!["investigate", "fish", "aquatic_creature"],
            TerrainFeatureType::Waterfall => vec!["behind_falls", "collect_water", "meditation"],
            TerrainFeatureType::Pond => vec!["ritual", "gather_ingredients"],
            TerrainFeatureType::Stream => vec!["follow_source", "purify"],
            TerrainFeatureType::Forest => {
                vec!["hunt", "gather_herbs", "clear_path", "lost_explorer"]
            }
            TerrainFeatureType::Grove => vec!["sacred_ritual", "spirit_encounter", "harvest"],
            TerrainFeatureType::Meadow => vec!["gather_flowers", "peaceful_encounter", "camp"],
            TerrainFeatureType::Marsh => vec!["navigate", "creature_hunt", "lost_artifact"],
            TerrainFeatureType::Desert => vec!["survive", "find_oasis", "ancient_ruins"],
            TerrainFeatureType::Cave => vec!["explore_depths", "monster_lair", "hidden_treasure"],
            TerrainFeatureType::Crater => vec!["investigate_impact", "alien_artifact", "mining"],
            TerrainFeatureType::Plateau => {
                vec!["establish_base", "aerial_view", "forgotten_temple"]
            }
            TerrainFeatureType::Ridge => vec!["traverse", "lookout", "eagle_nest"],
            TerrainFeatureType::Ravine => vec!["descend", "ancient_path", "hidden_cave"],
        }
    }

    /// Get difficulty modifier for this terrain type
    pub fn difficulty_modifier(&self) -> f32 {
        match self {
            TerrainFeatureType::Mountain => 0.3,
            TerrainFeatureType::Cliff => 0.25,
            TerrainFeatureType::Canyon => 0.2,
            TerrainFeatureType::Cave => 0.25,
            TerrainFeatureType::Marsh => 0.15,
            TerrainFeatureType::Desert => 0.2,
            TerrainFeatureType::Crater => 0.15,
            TerrainFeatureType::Ravine => 0.2,
            TerrainFeatureType::Waterfall => 0.1,
            TerrainFeatureType::Lake => 0.05,
            TerrainFeatureType::River => 0.1,
            TerrainFeatureType::Hill => 0.05,
            TerrainFeatureType::Valley => 0.0,
            TerrainFeatureType::Forest => 0.1,
            TerrainFeatureType::Grove => 0.0,
            TerrainFeatureType::Meadow => -0.05,
            TerrainFeatureType::Pond => 0.0,
            TerrainFeatureType::Stream => 0.0,
            TerrainFeatureType::Plateau => 0.1,
            TerrainFeatureType::Ridge => 0.15,
        }
    }
}

/// Context for terrain-driven quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainQuestContext {
    /// Type of terrain feature
    pub feature_type: TerrainFeatureType,
    /// World position of the feature (x, y, z)
    pub position: (f32, f32, f32),
    /// Size/radius of the feature
    pub radius: f32,
    /// Intensity of the feature (0.0-1.0)
    pub intensity: f32,
    /// Biome where feature exists
    pub biome: String,
    /// Nearby notable features
    pub nearby_features: Vec<String>,
    /// Whether this is an AI-generated feature
    pub is_ai_generated: bool,
    /// Generation seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for TerrainQuestContext {
    fn default() -> Self {
        Self {
            feature_type: TerrainFeatureType::Hill,
            position: (0.0, 0.0, 0.0),
            radius: 32.0,
            intensity: 0.5,
            biome: "unknown".to_string(),
            nearby_features: vec![],
            is_ai_generated: false,
            seed: None,
        }
    }
}

/// Trigger condition for terrain-based quests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainQuestTrigger {
    /// Unique ID for this trigger
    pub id: String,
    /// Feature types that activate this trigger
    pub feature_types: Vec<TerrainFeatureType>,
    /// Minimum player level to trigger
    pub min_player_level: u32,
    /// Maximum player level (None = no limit)
    pub max_player_level: Option<u32>,
    /// Biome requirements
    pub required_biomes: Vec<String>,
    /// Probability of triggering (0.0-1.0)
    pub trigger_probability: f32,
    /// Cooldown in seconds before this can trigger again
    pub cooldown_seconds: f32,
    /// Quest template to use
    pub quest_template: String,
}

impl TerrainQuestTrigger {
    /// Check if this trigger should activate
    pub fn should_trigger(
        &self,
        terrain_ctx: &TerrainQuestContext,
        player_level: u32,
        _rng: &mut impl rand::Rng,
    ) -> bool {
        // Check feature type
        if !self.feature_types.contains(&terrain_ctx.feature_type) {
            return false;
        }

        // Check player level
        if player_level < self.min_player_level {
            return false;
        }
        if let Some(max) = self.max_player_level {
            if player_level > max {
                return false;
            }
        }

        // Check biome
        if !self.required_biomes.is_empty() && !self.required_biomes.contains(&terrain_ctx.biome) {
            return false;
        }

        // Random roll - using simple deterministic check for now
        // Real implementation would use rng.gen::<f32>() < self.trigger_probability
        true
    }
}

/// Generated terrain quest with location binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainQuest {
    /// The base quest
    pub quest: LlmQuest,
    /// Terrain context that triggered this quest
    pub terrain_context: TerrainQuestContext,
    /// Whether quest is bound to terrain location
    pub location_bound: bool,
    /// Terrain-specific objectives
    pub terrain_objectives: Vec<TerrainObjective>,
}

/// Terrain-specific quest objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainObjective {
    /// Objective ID
    pub id: String,
    /// Description
    pub description: String,
    /// Required terrain interaction
    pub interaction_type: TerrainInteraction,
    /// Target position
    pub target_position: (f32, f32, f32),
    /// Completion radius
    pub completion_radius: f32,
    /// Is completed
    pub completed: bool,
}

/// Types of terrain interactions for quest objectives
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TerrainInteraction {
    /// Player must reach this location
    Reach,
    /// Player must collect something here
    Collect,
    /// Player must defeat enemies here
    Combat,
    /// Player must investigate/interact
    Investigate,
    /// Player must build or modify something
    Build,
    /// Player must survive in this terrain
    Survive,
    /// Player must escort someone through
    Escort,
}

/// Configuration for terrain quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainQuestConfig {
    /// Base quest config
    pub quest_config: QuestGenerationConfig,
    /// Enable terrain-driven quests
    pub enabled: bool,
    /// Maximum terrain quests active at once
    pub max_active_quests: usize,
    /// Minimum distance between terrain quest locations
    pub min_quest_spacing: f32,
    /// Bonus rewards for AI-generated terrain quests
    pub ai_terrain_reward_bonus: f32,
}

impl Default for TerrainQuestConfig {
    fn default() -> Self {
        Self {
            quest_config: QuestGenerationConfig::default(),
            enabled: true,
            max_active_quests: 5,
            min_quest_spacing: 100.0,
            ai_terrain_reward_bonus: 1.25,
        }
    }
}

/// Terrain quest generator
pub struct TerrainQuestGenerator {
    /// Configuration
    config: TerrainQuestConfig,
    /// Registered triggers
    triggers: Vec<TerrainQuestTrigger>,
    /// Active terrain quests
    active_quests: Vec<TerrainQuest>,
    /// Quest generation counter
    quests_generated: u32,
}

impl TerrainQuestGenerator {
    /// Create a new terrain quest generator
    pub fn new(config: TerrainQuestConfig) -> Self {
        Self {
            config,
            triggers: Self::default_triggers(),
            active_quests: vec![],
            quests_generated: 0,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(TerrainQuestConfig::default())
    }

    /// Get default triggers for common terrain types
    fn default_triggers() -> Vec<TerrainQuestTrigger> {
        vec![
            TerrainQuestTrigger {
                id: "mountain_expedition".into(),
                feature_types: vec![TerrainFeatureType::Mountain],
                min_player_level: 5,
                max_player_level: None,
                required_biomes: vec![],
                trigger_probability: 0.3,
                cooldown_seconds: 300.0,
                quest_template: "mountain_exploration".into(),
            },
            TerrainQuestTrigger {
                id: "cave_discovery".into(),
                feature_types: vec![TerrainFeatureType::Cave],
                min_player_level: 3,
                max_player_level: Some(20),
                required_biomes: vec![],
                trigger_probability: 0.5,
                cooldown_seconds: 180.0,
                quest_template: "cave_exploration".into(),
            },
            TerrainQuestTrigger {
                id: "forest_mystery".into(),
                feature_types: vec![TerrainFeatureType::Forest, TerrainFeatureType::Grove],
                min_player_level: 1,
                max_player_level: None,
                required_biomes: vec![],
                trigger_probability: 0.4,
                cooldown_seconds: 120.0,
                quest_template: "forest_exploration".into(),
            },
            TerrainQuestTrigger {
                id: "water_journey".into(),
                feature_types: vec![
                    TerrainFeatureType::River,
                    TerrainFeatureType::Lake,
                    TerrainFeatureType::Waterfall,
                ],
                min_player_level: 1,
                max_player_level: None,
                required_biomes: vec![],
                trigger_probability: 0.35,
                cooldown_seconds: 150.0,
                quest_template: "water_exploration".into(),
            },
        ]
    }

    /// Register a custom trigger
    pub fn register_trigger(&mut self, trigger: TerrainQuestTrigger) {
        self.triggers.push(trigger);
    }

    /// Generate a quest for a terrain feature
    pub fn generate_quest(
        &mut self,
        terrain_ctx: &TerrainQuestContext,
        player_ctx: &QuestContext,
    ) -> Result<Option<TerrainQuest>> {
        if !self.config.enabled {
            return Ok(None);
        }

        // Check active quest limit
        if self.active_quests.len() >= self.config.max_active_quests {
            debug!("Max active terrain quests reached");
            return Ok(None);
        }

        // Check spacing
        for active in &self.active_quests {
            let (ax, ay, az) = active.terrain_context.position;
            let (tx, ty, tz) = terrain_ctx.position;
            let dist = ((ax - tx).powi(2) + (ay - ty).powi(2) + (az - tz).powi(2)).sqrt();
            if dist < self.config.min_quest_spacing {
                debug!("Terrain quest too close to existing quest");
                return Ok(None);
            }
        }

        // Find matching trigger
        let _trigger = self
            .triggers
            .iter()
            .find(|t| t.feature_types.contains(&terrain_ctx.feature_type));

        // Generate quest
        let quest = self.create_terrain_quest(terrain_ctx, player_ctx)?;

        info!(
            "Generated terrain quest '{}' for {:?} at ({:.1}, {:.1}, {:.1})",
            quest.quest.title,
            terrain_ctx.feature_type,
            terrain_ctx.position.0,
            terrain_ctx.position.1,
            terrain_ctx.position.2
        );

        self.quests_generated += 1;
        self.active_quests.push(quest.clone());

        Ok(Some(quest))
    }

    /// Create a terrain-bound quest
    fn create_terrain_quest(
        &self,
        terrain_ctx: &TerrainQuestContext,
        player_ctx: &QuestContext,
    ) -> Result<TerrainQuest> {
        let archetypes = terrain_ctx.feature_type.quest_archetypes();
        let archetype = archetypes.first().unwrap_or(&"explore");

        let title = format!(
            "{} of the {}",
            Self::capitalize(archetype),
            Self::feature_name(terrain_ctx.feature_type)
        );

        let description = format!(
            "A mysterious {} has appeared in {}. Investigate this new terrain feature and uncover its secrets.",
            Self::feature_description(terrain_ctx.feature_type),
            terrain_ctx.biome
        );

        // Calculate difficulty
        let base_difficulty = 0.3 + (player_ctx.player_level as f32 * 0.02);
        let terrain_modifier = terrain_ctx.feature_type.difficulty_modifier();
        let difficulty =
            (base_difficulty + terrain_modifier + terrain_ctx.intensity * 0.2).clamp(0.0, 1.0);

        // Create objectives
        let terrain_objectives = vec![
            TerrainObjective {
                id: "reach_feature".into(),
                description: format!(
                    "Reach the {} at coordinates ({:.0}, {:.0})",
                    Self::feature_name(terrain_ctx.feature_type),
                    terrain_ctx.position.0,
                    terrain_ctx.position.2
                ),
                interaction_type: TerrainInteraction::Reach,
                target_position: terrain_ctx.position,
                completion_radius: terrain_ctx.radius,
                completed: false,
            },
            TerrainObjective {
                id: "investigate".into(),
                description: format!(
                    "Investigate the {} to discover its secrets",
                    Self::feature_name(terrain_ctx.feature_type)
                ),
                interaction_type: TerrainInteraction::Investigate,
                target_position: terrain_ctx.position,
                completion_radius: terrain_ctx.radius / 2.0,
                completed: false,
            },
        ];

        // Create LLM quest structure
        let quest_id = format!("terrain_{}", self.quests_generated + 1);
        let llm_quest = LlmQuest {
            id: quest_id,
            title: title.clone(),
            description,
            steps: vec![
                LlmQuestStep {
                    id: "step_1".into(),
                    description: format!(
                        "Travel to the {} and reach its location",
                        Self::feature_name(terrain_ctx.feature_type)
                    ),
                    completed: false,
                    objectives: vec![QuestObjective {
                        id: "obj_1".into(),
                        description: "Reach the terrain feature".into(),
                        objective_type: ObjectiveType::Reach,
                        target_count: Some(1),
                        current_count: 0,
                        completion_criteria: "Arrive at the location".into(),
                    }],
                    branching_choices: vec![],
                    dynamic_content: None,
                    validation_criteria: ValidationCriteria {
                        required_conditions: vec!["player_near_location".into()],
                        forbidden_conditions: vec![],
                        validation_script: None,
                    },
                },
                LlmQuestStep {
                    id: "step_2".into(),
                    description: "Investigate the terrain feature".into(),
                    completed: false,
                    objectives: vec![QuestObjective {
                        id: "obj_2".into(),
                        description: "Investigate the feature".into(),
                        objective_type: ObjectiveType::Explore,
                        target_count: Some(1),
                        current_count: 0,
                        completion_criteria: "Complete investigation".into(),
                    }],
                    branching_choices: vec![],
                    dynamic_content: None,
                    validation_criteria: ValidationCriteria {
                        required_conditions: vec!["investigation_complete".into()],
                        forbidden_conditions: vec![],
                        validation_script: None,
                    },
                },
            ],
            metadata: QuestMetadata {
                category: format!("{:?}", terrain_ctx.feature_type),
                difficulty_level: difficulty,
                estimated_duration: (15.0 + difficulty * 30.0) as u32,
                player_level_range: (
                    player_ctx.player_level.saturating_sub(2),
                    player_ctx.player_level + 3,
                ),
                required_skills: vec!["exploration".into()],
                tags: vec![
                    "terrain".into(),
                    format!("{:?}", terrain_ctx.feature_type).to_lowercase(),
                    terrain_ctx.biome.clone(),
                ],
                generated_reasoning: format!(
                    "Quest generated for AI-created {} terrain feature",
                    Self::feature_name(terrain_ctx.feature_type)
                ),
            },
            branching: crate::QuestBranching {
                has_multiple_paths: false,
                branch_points: vec![],
                convergence_points: vec![],
            },
            rewards: QuestRewards {
                experience: self.calculate_experience(difficulty, terrain_ctx.is_ai_generated),
                currency: self.calculate_currency(difficulty, terrain_ctx.is_ai_generated),
                items: vec![],
                reputation_changes: HashMap::new(),
                unlock_content: vec![],
            },
            generated_at: 0,
            personalization: crate::PersonalizationData {
                player_id: player_ctx.player_id.clone(),
                player_preferences: player_ctx.preferred_quest_types.clone(),
                play_style: "Explorer".into(),
                previous_choices: vec![],
                difficulty_preference: difficulty,
            },
        };

        Ok(TerrainQuest {
            quest: llm_quest,
            terrain_context: terrain_ctx.clone(),
            location_bound: true,
            terrain_objectives,
        })
    }

    /// Calculate experience reward
    fn calculate_experience(&self, difficulty: f32, is_ai_generated: bool) -> u32 {
        let base = 100.0 + difficulty * 400.0;
        let multiplier = if is_ai_generated {
            self.config.ai_terrain_reward_bonus
        } else {
            1.0
        };
        (base * multiplier) as u32
    }

    /// Calculate currency reward
    fn calculate_currency(&self, difficulty: f32, is_ai_generated: bool) -> u32 {
        let base = 10.0 + difficulty * 40.0;
        let multiplier = if is_ai_generated {
            self.config.ai_terrain_reward_bonus
        } else {
            1.0
        };
        (base * multiplier) as u32
    }

    fn capitalize(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    fn feature_name(feature: TerrainFeatureType) -> String {
        format!("{:?}", feature).to_lowercase()
    }

    fn feature_description(feature: TerrainFeatureType) -> &'static str {
        match feature {
            TerrainFeatureType::Mountain => "towering mountain",
            TerrainFeatureType::Hill => "rolling hill",
            TerrainFeatureType::Valley => "sheltered valley",
            TerrainFeatureType::Cave => "mysterious cave",
            TerrainFeatureType::Forest => "dense forest",
            TerrainFeatureType::Lake => "serene lake",
            TerrainFeatureType::River => "flowing river",
            TerrainFeatureType::Waterfall => "cascading waterfall",
            _ => "strange terrain feature",
        }
    }

    /// Get active quests
    pub fn active_quests(&self) -> &[TerrainQuest] {
        &self.active_quests
    }

    /// Complete a terrain quest
    pub fn complete_quest(&mut self, quest_id: &str) -> Option<TerrainQuest> {
        if let Some(idx) = self
            .active_quests
            .iter()
            .position(|q| q.quest.id == quest_id)
        {
            Some(self.active_quests.remove(idx))
        } else {
            None
        }
    }

    /// Get stats
    pub fn quests_generated(&self) -> u32 {
        self.quests_generated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_feature_archetypes() {
        let archetypes = TerrainFeatureType::Mountain.quest_archetypes();
        assert!(!archetypes.is_empty());
        assert!(archetypes.contains(&"climb"));
    }

    #[test]
    fn test_terrain_feature_difficulty() {
        assert!(TerrainFeatureType::Mountain.difficulty_modifier() > 0.0);
        assert!(TerrainFeatureType::Meadow.difficulty_modifier() < 0.0);
    }

    #[test]
    fn test_terrain_quest_context_default() {
        let ctx = TerrainQuestContext::default();
        assert_eq!(ctx.feature_type, TerrainFeatureType::Hill);
        assert!(!ctx.is_ai_generated);
    }

    #[test]
    fn test_terrain_quest_generator_creation() {
        let gen = TerrainQuestGenerator::default_config();
        assert!(gen.config.enabled);
        assert!(!gen.triggers.is_empty());
    }

    #[test]
    fn test_generate_terrain_quest() {
        let mut gen = TerrainQuestGenerator::default_config();

        let terrain_ctx = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (100.0, 50.0, 200.0),
            radius: 32.0,
            intensity: 0.7,
            biome: "forest".into(),
            nearby_features: vec![],
            is_ai_generated: true,
            seed: Some(12345),
        };

        let player_ctx = QuestContext {
            player_id: "player_1".into(),
            player_level: 10,
            location: "forest".into(),
            available_npcs: vec![],
            world_state: HashMap::new(),
            recent_activities: vec![],
            preferred_quest_types: vec!["exploration".into()],
        };

        let result = gen.generate_quest(&terrain_ctx, &player_ctx);
        assert!(result.is_ok());

        let quest = result.unwrap();
        assert!(quest.is_some());

        let terrain_quest = quest.unwrap();
        assert!(
            terrain_quest.quest.title.to_lowercase().contains("cave"),
            "Expected title to contain 'cave', got: {}",
            terrain_quest.quest.title
        );
        assert!(terrain_quest.location_bound);
        assert!(!terrain_quest.terrain_objectives.is_empty());
    }

    #[test]
    fn test_terrain_quest_rewards_bonus() {
        let gen = TerrainQuestGenerator::default_config();

        let ai_exp = gen.calculate_experience(0.5, true);
        let normal_exp = gen.calculate_experience(0.5, false);

        assert!(ai_exp > normal_exp);
    }

    #[test]
    fn test_quest_spacing() {
        let mut gen = TerrainQuestGenerator::default_config();

        let terrain_ctx1 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Cave,
            position: (100.0, 0.0, 100.0),
            ..Default::default()
        };

        let terrain_ctx2 = TerrainQuestContext {
            feature_type: TerrainFeatureType::Forest,
            position: (110.0, 0.0, 110.0), // Too close
            ..Default::default()
        };

        let player_ctx = QuestContext {
            player_id: "player_1".into(),
            player_level: 5,
            location: "forest".into(),
            available_npcs: vec![],
            world_state: HashMap::new(),
            recent_activities: vec![],
            preferred_quest_types: vec![],
        };

        // First quest should succeed
        let result1 = gen.generate_quest(&terrain_ctx1, &player_ctx);
        assert!(result1.is_ok());
        assert!(result1.unwrap().is_some());

        // Second quest too close should fail
        let result2 = gen.generate_quest(&terrain_ctx2, &player_ctx);
        assert!(result2.is_ok());
        assert!(result2.unwrap().is_none());
    }
}
