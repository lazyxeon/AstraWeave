use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use astraweave_context::ConversationHistory;
use astraweave_llm::LlmClient;
use astraweave_prompts::library::PromptLibrary;
use astraweave_prompts::template::PromptTemplate;
use astraweave_rag::RagPipeline;

use crate::{Quest, QuestStep};

/// Enhanced quest structure with LLM generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQuest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<LlmQuestStep>,
    pub metadata: QuestMetadata,
    pub branching: QuestBranching,
    pub rewards: QuestRewards,
    /// Sim tick that accepted the quest. `0` means the model proposed it and the sim has not stamped it.
    pub generated_at: u64,
    pub personalization: PersonalizationData,
}

/// Enhanced quest step with dynamic content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmQuestStep {
    pub id: String,
    pub description: String,
    pub completed: bool,
    pub objectives: Vec<QuestObjective>,
    pub branching_choices: Vec<BranchingChoice>,
    pub dynamic_content: Option<DynamicContent>,
    pub validation_criteria: ValidationCriteria,
}

/// Quest generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestMetadata {
    pub category: String,
    pub difficulty_level: f32,   // 0.0 to 1.0
    pub estimated_duration: u32, // minutes
    pub player_level_range: (u32, u32),
    pub required_skills: Vec<String>,
    pub tags: Vec<String>,
    pub generated_reasoning: String,
}

/// Quest branching system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestBranching {
    pub has_multiple_paths: bool,
    pub branch_points: Vec<BranchPoint>,
    pub convergence_points: Vec<String>, // Step IDs where branches rejoin
}

/// Individual branch point in quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchPoint {
    pub step_id: String,
    pub condition: String,
    pub branches: Vec<QuestBranch>,
}

/// A branch in the quest narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestBranch {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<String>, // Step IDs
    pub consequences: Vec<String>,
}

/// Quest rewards structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestRewards {
    pub experience: u32,
    pub currency: u32,
    pub items: Vec<String>,
    pub reputation_changes: HashMap<String, i32>,
    pub unlock_content: Vec<String>,
}

/// Personalization data for quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationData {
    pub player_id: String,
    pub player_preferences: Vec<String>,
    pub play_style: String,
    pub previous_choices: Vec<String>,
    pub difficulty_preference: f32,
}

/// Quest objective with specific goals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    pub id: String,
    pub description: String,
    pub objective_type: ObjectiveType,
    pub target_count: Option<u32>,
    pub current_count: u32,
    pub completion_criteria: String,
}

/// Types of quest objectives
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ObjectiveType {
    Collect,
    Defeat,
    Interact,
    Reach,
    Deliver,
    Craft,
    Explore,
    Survive,
    Custom(String),
}

/// Player choice in quest branching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchingChoice {
    pub id: String,
    pub description: String,
    pub consequences: Vec<String>,
    pub requirements: Option<String>,
    pub leads_to_step: Option<String>,
}

/// Dynamic content that can be regenerated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicContent {
    pub dialogue: Option<String>,
    pub flavor_text: Option<String>,
    pub environmental_description: Option<String>,
    pub npc_interactions: Vec<String>,
}

/// Validation criteria for quest steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriteria {
    pub required_conditions: Vec<String>,
    pub forbidden_conditions: Vec<String>,
    pub validation_script: Option<String>,
}

/// Context for quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestContext {
    pub player_id: String,
    pub player_level: u32,
    pub location: String,
    pub available_npcs: Vec<String>,
    pub world_state: HashMap<String, serde_json::Value>,
    pub recent_activities: Vec<String>,
    pub preferred_quest_types: Vec<String>,
}

/// Quest generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestGenerationConfig {
    pub creativity_level: f32,
    pub personalization_weight: f32,
    pub branching_complexity: f32,
    pub max_steps: u32,
    pub enable_dynamic_content: bool,
    pub validation_strictness: f32,
    pub context_window_size: usize,
}

impl Default for QuestGenerationConfig {
    fn default() -> Self {
        Self {
            creativity_level: 0.7,
            personalization_weight: 0.8,
            branching_complexity: 0.6,
            max_steps: 10,
            enable_dynamic_content: true,
            validation_strictness: 0.8,
            context_window_size: 2048,
        }
    }
}

/// LLM-powered quest generator
pub struct LlmQuestGenerator {
    llm_client: Arc<dyn LlmClient>,
    rag_pipeline: Arc<RagPipeline>,
    #[allow(dead_code)]
    conversation_history: Arc<RwLock<ConversationHistory>>,
    prompt_library: Arc<RwLock<PromptLibrary>>,
    config: QuestGenerationConfig,
    #[allow(dead_code)]
    quest_templates: Arc<RwLock<HashMap<String, QuestTemplate>>>,
}

/// Template for quest generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestTemplate {
    pub name: String,
    pub category: String,
    pub structure: QuestStructure,
    pub example_content: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestStructure {
    pub typical_steps: u32,
    pub branching_points: u32,
    pub complexity_level: f32,
}

impl LlmQuestGenerator {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        rag_pipeline: Arc<RagPipeline>,
        config: QuestGenerationConfig,
    ) -> Result<Self> {
        let context_config = astraweave_context::ContextConfig {
            max_tokens: config.context_window_size,
            ..Default::default()
        };
        let conversation_history = Arc::new(RwLock::new(ConversationHistory::new(context_config)));

        let mut prompt_library = PromptLibrary::new();

        // Load quest generation prompts
        prompt_library.add_template("quest_generation", PromptTemplate::new("quest_generation".to_string(),
            r#"
You are a creative quest designer for a dynamic fantasy game. Generate an engaging quest based on the provided context and player history.

Player Context:
- ID: {{player_id}}
- Level: {{player_level}}
- Location: {{location}}
- Play Style: {{play_style}}
- Previous Quest Types: {{previous_quests}}

Player History: {{player_history}}

World State: {{world_state}}

Available NPCs: {{available_npcs}}

Generate a quest in JSON format with the following structure:
{
    "title": "Quest Title",
    "description": "Compelling quest description",
    "steps": [
        {
            "id": "step_1",
            "description": "Step description",
            "objectives": [
                {
                    "id": "obj_1",
                    "description": "Objective description",
                    "objective_type": "Collect",
                    "target_count": 5,
                    "completion_criteria": "Collect 5 magical herbs"
                }
            ],
            "branching_choices": [
                {
                    "id": "choice_1",
                    "description": "Choice description",
                    "consequences": ["Consequence 1", "Consequence 2"]
                }
            ]
        }
    ],
    "metadata": {
        "category": "Collection",
        "difficulty_level": 0.7,
        "estimated_duration": 45,
        "player_level_range": [10, 15],
        "required_skills": ["Herbalism", "Combat"],
        "tags": ["Magic", "Nature", "Exploration"],
        "generated_reasoning": "Why this quest fits the player"
    },
    "branching": {
        "has_multiple_paths": true,
        "branch_points": [...]
    },
    "rewards": {
        "experience": 1000,
        "currency": 50,
        "items": ["Magic Potion"],
        "reputation_changes": {"Herbalists Guild": 10}
    }
}

Guidelines:
1. Personalize based on player history and preferences
2. Create meaningful choices with consequences
3. Balance challenge with player level
4. Include rich narrative elements
5. Ensure quest steps are achievable
6. Add branching for replay value
            "#.trim().to_string()
        ));

        prompt_library.add_template("quest_branching", PromptTemplate::new("quest_branching".to_string(),
            r#"
You are expanding a quest with branching narrative paths. Based on the current quest state and player choice, generate the next branch.

Current Quest: {{current_quest}}
Player Choice: {{player_choice}}
Player Preferences: {{player_preferences}}

Generate the branching path in JSON format:
{
    "branch_id": "unique_branch_id",
    "name": "Branch Name",
    "description": "What happens based on player choice",
    "steps": [
        // New quest steps for this branch
    ],
    "consequences": [
        "Long-term consequence 1",
        "Short-term consequence 2"
    ],
    "convergence_point": "step_id_where_branches_rejoin" // optional
}

Consider:
- Player agency and meaningful choice impact
- Narrative coherence with main quest
- Balanced outcomes regardless of choice
- Future quest opportunities
            "#.trim().to_string()
        ));

        prompt_library.add_template(
            "quest_validation",
            PromptTemplate::new(
                "quest_validation".to_string(),
                r#"
You are validating a generated quest for quality, coherence, and balance.

Quest to Validate: {{quest_json}}
World Context: {{world_context}}
Player Context: {{player_context}}

Provide validation results in JSON format:
{
    "is_valid": true,
    "quality_score": 0.85,
    "issues": [
        {
            "type": "balance",
            "severity": "minor",
            "description": "Rewards may be too high for quest difficulty",
            "suggestion": "Reduce currency reward by 20%"
        }
    ],
    "strengths": [
        "Strong narrative integration",
        "Good personalization"
    ],
    "overall_assessment": "High quality quest with minor balance adjustments needed"
}

Validation Criteria:
- Narrative coherence
- Balance (difficulty vs rewards)
- Player engagement potential
- Technical feasibility
- World consistency
            "#
                .trim()
                .to_string(),
            ),
        );

        Ok(Self {
            llm_client,
            rag_pipeline,
            conversation_history,
            prompt_library: Arc::new(RwLock::new(prompt_library)),
            config,
            quest_templates: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Generate a new quest based on player context and history
    pub async fn generate_quest(&self, context: &QuestContext) -> Result<LlmQuest> {
        debug!("Generating quest for player {}", context.player_id);

        // Retrieve player quest history from RAG
        let player_history_raw = self
            .rag_pipeline
            .retrieve(&format!("player:{} quests", context.player_id), 10)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to retrieve player history: {}", e);
                Vec::new()
            });
        let player_history: Vec<String> = player_history_raw
            .iter()
            .map(|m| m.memory.text.clone())
            .collect();

        // Build generation context
        let generation_context = self
            .build_generation_context(context, &player_history)
            .await?;

        // Generate quest using LLM
        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("quest_generation")?;
        let prompt = template.render_map(&generation_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Quest generation failed: {}", e))?;

        // Parse generated quest
        let mut quest: LlmQuest = astraweave_llm::parse_bounded_model_json(&response)
            .map_err(|e| anyhow!("Failed to parse generated quest: {}", e))?;

        // Add metadata
        quest.id = Uuid::new_v4().to_string();
        quest.generated_at = 0;
        quest.personalization = PersonalizationData {
            player_id: context.player_id.clone(),
            player_preferences: context.preferred_quest_types.clone(),
            play_style: self.infer_play_style(&player_history),
            previous_choices: player_history.clone(),
            difficulty_preference: self.infer_difficulty_preference(&player_history),
        };

        // Validate quest
        let validation = self.validate_quest(&quest, context).await?;
        if !validation.is_valid {
            warn!("Generated quest failed validation: {:?}", validation.issues);
            return Err(anyhow!("Quest validation failed"));
        }

        // Store quest in RAG for future reference
        let quest_summary = format!(
            "Quest: {} | Category: {} | Difficulty: {:.2} | Player: {}",
            quest.title,
            quest.metadata.category,
            quest.metadata.difficulty_level,
            context.player_id
        );
        // Store quest summary as memory - best-effort if we have unique ownership
        let mut cloned_pipeline = self.rag_pipeline.clone();
        if let Some(inner) = Arc::get_mut(&mut cloned_pipeline) {
            let _ = inner.add_memory(quest_summary).await;
        } else {
            // cannot obtain mutable access; skip storing to avoid panics
        }

        info!(
            "Generated quest '{}' for player {}",
            quest.title, context.player_id
        );
        Ok(quest)
    }

    /// Branch narrative based on player choice
    pub async fn branch_narrative(
        &self,
        quest: &LlmQuest,
        choice: &BranchingChoice,
        context: &QuestContext,
    ) -> Result<QuestBranch> {
        debug!(
            "Branching quest narrative for choice: {}",
            choice.description
        );

        let branch_context = HashMap::from([
            ("current_quest".to_string(), serde_json::to_string(quest)?),
            ("player_choice".to_string(), serde_json::to_string(choice)?),
            (
                "player_preferences".to_string(),
                context.preferred_quest_types.join(", "),
            ),
        ]);

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("quest_branching")?;
        let prompt = template.render_map(&branch_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Quest branching failed: {}", e))?;

        let branch: QuestBranch = astraweave_llm::parse_bounded_model_json(&response)
            .map_err(|e| anyhow!("Failed to parse quest branch: {}", e))?;

        Ok(branch)
    }

    /// Validate generated quest for quality and coherence
    pub async fn validate_quest(
        &self,
        quest: &LlmQuest,
        context: &QuestContext,
    ) -> Result<QuestValidation> {
        debug!("Validating quest: {}", quest.title);

        let validation_context = HashMap::from([
            ("quest_json".to_string(), serde_json::to_string(quest)?),
            (
                "world_context".to_string(),
                serde_json::to_string(&context.world_state)?,
            ),
            (
                "player_context".to_string(),
                serde_json::to_string(context)?,
            ),
        ]);

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("quest_validation")?;
        let prompt = template.render_map(&validation_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Quest validation failed: {}", e))?;

        let validation: QuestValidation = astraweave_llm::parse_bounded_model_json(&response)
            .map_err(|e| anyhow!("Failed to parse validation result: {}", e))?;

        Ok(validation)
    }

    /// Generate dynamic content for quest step
    pub async fn generate_dynamic_content(
        &self,
        step: &LlmQuestStep,
        context: &QuestContext,
    ) -> Result<DynamicContent> {
        if !self.config.enable_dynamic_content {
            return Ok(DynamicContent {
                dialogue: None,
                flavor_text: None,
                environmental_description: None,
                npc_interactions: Vec::new(),
            });
        }

        // Generate contextual content based on step and world state
        let content_prompt = format!(
            "Generate dynamic content for quest step '{}' in location '{}' with NPCs: {:?}",
            step.description, context.location, context.available_npcs
        );

        let response = self
            .llm_client
            .complete(&content_prompt)
            .await
            .map_err(|e| anyhow!("Dynamic content generation failed: {}", e))?;

        // Parse and structure the response
        let content = DynamicContent {
            dialogue: Some(response),
            flavor_text: Some(format!("Dynamic content for {}", step.description)),
            environmental_description: Some(format!(
                "Environmental context for {}",
                context.location
            )),
            npc_interactions: context.available_npcs.clone(),
        };

        Ok(content)
    }

    /// Update quest based on player progress and choices
    pub async fn update_quest_progress(
        &self,
        quest: &mut LlmQuest,
        step_id: &str,
        choice: Option<&BranchingChoice>,
    ) -> Result<()> {
        // Find and complete the step
        if let Some(step) = quest.steps.iter_mut().find(|s| s.id == step_id) {
            step.completed = true;

            // Handle branching choice if provided
            if let Some(choice) = choice {
                if let Some(next_step_id) = &choice.leads_to_step {
                    // Activate next step based on choice
                    if let Some(next_step) = quest.steps.iter_mut().find(|s| s.id == *next_step_id)
                    {
                        info!(
                            "Player chose '{}', activating step '{}'",
                            choice.description, next_step.description
                        );
                    }
                }
            }
        }

        // Update quest completion status
        if quest.steps.iter().all(|s| s.completed) {
            info!("Quest '{}' completed!", quest.title);
        }

        Ok(())
    }

    /// Convert LlmQuest to basic Quest for compatibility
    pub fn to_basic_quest(&self, llm_quest: &LlmQuest) -> Quest {
        Quest {
            title: llm_quest.title.clone(),
            steps: llm_quest
                .steps
                .iter()
                .map(|step| QuestStep {
                    description: step.description.clone(),
                    completed: step.completed,
                })
                .collect(),
        }
    }

    /// Build context for quest generation
    async fn build_generation_context(
        &self,
        context: &QuestContext,
        player_history: &[String],
    ) -> Result<HashMap<String, String>> {
        let mut generation_context = HashMap::new();

        generation_context.insert("player_id".to_string(), context.player_id.clone());
        generation_context.insert("player_level".to_string(), context.player_level.to_string());
        generation_context.insert("location".to_string(), context.location.clone());
        generation_context.insert(
            "play_style".to_string(),
            self.infer_play_style(player_history),
        );
        generation_context.insert("previous_quests".to_string(), player_history.join(", "));
        generation_context.insert("player_history".to_string(), player_history.join(" | "));
        generation_context.insert(
            "world_state".to_string(),
            serde_json::to_string(&context.world_state)?,
        );
        generation_context.insert(
            "available_npcs".to_string(),
            context.available_npcs.join(", "),
        );

        Ok(generation_context)
    }

    /// Infer player's play style from history
    fn infer_play_style(&self, history: &[String]) -> String {
        // Simple heuristic - in practice would use more sophisticated analysis
        if history.iter().any(|h| h.contains("combat")) {
            "Combat-oriented".to_string()
        } else if history.iter().any(|h| h.contains("exploration")) {
            "Explorer".to_string()
        } else if history.iter().any(|h| h.contains("social")) {
            "Social".to_string()
        } else {
            "Balanced".to_string()
        }
    }

    /// Infer difficulty preference from history
    fn infer_difficulty_preference(&self, history: &[String]) -> f32 {
        // Simple heuristic - analyze success rates and quest types
        if history.len() < 3 {
            return 0.5; // Default medium difficulty
        }

        let hard_quests = history
            .iter()
            .filter(|h| h.contains("difficult") || h.contains("challenge"))
            .count();
        hard_quests as f32 / history.len() as f32
    }
}

/// Quest validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestValidation {
    pub is_valid: bool,
    pub quality_score: f32,
    pub issues: Vec<ValidationIssue>,
    pub strengths: Vec<String>,
    pub overall_assessment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub suggestion: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
    use astraweave_llm::MockLlm;
    use astraweave_rag::{RagConfig, RagPipeline, VectorStoreWrapper};

    fn create_test_rag_pipeline() -> Arc<RagPipeline> {
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
        let llm_client: Option<Arc<dyn astraweave_llm::LlmClient>> = Some(Arc::new(MockLlm));
        Arc::new(RagPipeline::new(
            embedding_client,
            vector_store,
            llm_client,
            RagConfig::default(),
        ))
    }

    #[tokio::test]
    async fn test_quest_generator_creation() {
        let llm_client: Arc<dyn astraweave_llm::LlmClient> = Arc::new(MockLlm);
        let rag_pipeline = create_test_rag_pipeline();
        let config = QuestGenerationConfig::default();

        let generator = LlmQuestGenerator::new(llm_client, rag_pipeline, config);
        assert!(generator.is_ok());
    }

    #[test]
    fn test_quest_objective_types() {
        let objective = QuestObjective {
            id: "test".to_string(),
            description: "Test objective".to_string(),
            objective_type: ObjectiveType::Collect,
            target_count: Some(5),
            current_count: 0,
            completion_criteria: "Collect 5 items".to_string(),
        };

        assert!(matches!(objective.objective_type, ObjectiveType::Collect));
    }

    #[test]
    fn test_play_style_inference() {
        let llm_client: Arc<dyn astraweave_llm::LlmClient> = Arc::new(MockLlm);
        let rag_pipeline = create_test_rag_pipeline();
        let generator =
            LlmQuestGenerator::new(llm_client, rag_pipeline, QuestGenerationConfig::default())
                .unwrap();

        let combat_history = vec!["combat quest completed".to_string()];
        assert_eq!(
            generator.infer_play_style(&combat_history),
            "Combat-oriented"
        );

        let exploration_history = vec!["exploration quest completed".to_string()];
        assert_eq!(generator.infer_play_style(&exploration_history), "Explorer");
    }
}
