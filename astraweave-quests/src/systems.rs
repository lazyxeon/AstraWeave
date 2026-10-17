use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::components::{CActiveQuest, CQuestGenerator, CQuestJournal, CQuestMetrics, QuestState};
use crate::llm_quests::{BranchingChoice, LlmQuestGenerator, QuestContext};

/// System for managing LLM-powered quest generation and execution
pub struct QuestLlmSystem {
    quest_generator: Arc<LlmQuestGenerator>,
    #[allow(dead_code)]
    generation_interval_ms: u64,
    #[allow(dead_code)]
    max_concurrent_quests: usize,
}

impl QuestLlmSystem {
    pub fn new(
        quest_generator: Arc<LlmQuestGenerator>,
        generation_interval_ms: u64,
        max_concurrent_quests: usize,
    ) -> Self {
        Self {
            quest_generator,
            generation_interval_ms,
            max_concurrent_quests,
        }
    }

    /// Main system update for quest management
    pub async fn update(
        &self,
        generator: &mut CQuestGenerator,
        active_quests: &mut Vec<CActiveQuest>,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
        current_time_ms: u64,
    ) -> Result<()> {
        // Check for quest generation
        if self.should_generate_quest(generator, journal, current_time_ms) {
            self.generate_quest(generator, metrics, journal, current_time_ms)
                .await?;
        }

        // Update active quests
        for active_quest in active_quests.iter_mut() {
            self.update_active_quest(active_quest, metrics, journal)
                .await?;
        }

        // Clean up completed/abandoned quests
        self.cleanup_quests(active_quests, metrics, journal);

        Ok(())
    }

    /// Generate a new quest for the player
    pub async fn generate_quest(
        &self,
        generator: &mut CQuestGenerator,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
        current_time_ms: u64,
    ) -> Result<()> {
        debug!(
            "Generating new quest for player {}",
            generator.context.player_id
        );

        if !journal.can_accept_new_quest() {
            debug!("Player has reached maximum active quest limit");
            return Ok(());
        }

        // Wall-clock generation cost is not sim state.
        let generation_time = 0.0;

        // Update context with learned preferences
        generator.context.preferred_quest_types = journal.get_preferred_categories();

        // Generate quest
        match self
            .quest_generator
            .generate_quest(&generator.context)
            .await
        {
            Ok(quest) => {
                // Validate quest
                let validation = self
                    .quest_generator
                    .validate_quest(&quest, &generator.context)
                    .await?;

                metrics.record_quest_generation(generation_time, true, Some(&validation));

                if validation.is_valid {
                    info!(
                        "Generated quest: '{}' (Quality: {:.2})",
                        quest.title, validation.quality_score
                    );

                    // Add to journal
                    journal.add_quest(&quest);

                    // Mark generation time
                    generator.mark_generation_time(current_time_ms);
                    generator.add_active_quest(quest.id.clone());

                    // Add to active quests for tracking
                    // Note: This would typically be handled by the ECS spawn system
                    debug!("Quest '{}' ready for activation", quest.id);
                } else {
                    warn!("Generated quest failed validation: {:?}", validation.issues);
                    metrics.record_quest_generation(generation_time, false, Some(&validation));
                }

                Ok(())
            }
            Err(e) => {
                metrics.record_quest_generation(0.0, false, None);
                error!("Failed to generate quest: {}", e);
                Err(e)
            }
        }
    }

    /// Update an active quest's progress
    async fn update_active_quest(
        &self,
        active_quest: &mut CActiveQuest,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
    ) -> Result<()> {
        if !matches!(active_quest.state, QuestState::Active) {
            return Ok(());
        }

        // Check if current step has dynamic content needs
        if let Some(current_step) = active_quest.get_current_step() {
            let step_id = current_step.id.clone();
            // Generate dynamic content if not cached
            if !active_quest.dynamic_content.contains_key(&step_id) {
                match self
                    .generate_dynamic_content_for_step(active_quest, step_id.clone())
                    .await
                {
                    Ok(content) => {
                        active_quest.add_dynamic_content(step_id.clone(), content);
                        debug!("Generated dynamic content for step '{}'", step_id);
                    }
                    Err(e) => {
                        warn!("Failed to generate dynamic content: {}", e);
                    }
                }
            }
        }

        // Check for quest completion
        if active_quest.is_complete() && !matches!(active_quest.state, QuestState::Completed) {
            self.complete_quest(active_quest, metrics, journal).await?;
        }

        Ok(())
    }

    /// Complete a quest
    async fn complete_quest(
        &self,
        active_quest: &mut CActiveQuest,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
    ) -> Result<()> {
        let tick = active_quest.last_update;
        active_quest.set_state(QuestState::Completed, tick);
        let duration = active_quest.get_duration();
        let completion_time_minutes = duration as f64;

        info!(
            "Quest '{}' completed in {:.1} minutes",
            active_quest.quest.title, completion_time_minutes
        );

        // Calculate quality score based on completion
        let quality_score = self.calculate_completion_quality(active_quest);

        // Update metrics
        metrics.record_quest_completion(
            &active_quest.quest,
            completion_time_minutes,
            quality_score,
        );

        // Update journal
        journal.complete_quest(
            &active_quest.quest.id,
            "Quest completed successfully".to_string(),
        );

        Ok(())
    }

    /// Handle player choice in quest branching
    pub async fn handle_player_choice(
        &self,
        active_quest: &mut CActiveQuest,
        choice: BranchingChoice,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
    ) -> Result<()> {
        let step_id = active_quest
            .get_current_step()
            .ok_or_else(|| anyhow::anyhow!("No current step for choice"))?
            .id
            .clone();

        info!(
            "Player chose '{}' in quest '{}'",
            choice.description, active_quest.quest.title
        );

        // Record choice
        active_quest.record_choice(step_id.clone(), choice.clone(), active_quest.last_update);
        journal.record_choice(&active_quest.quest.id, choice.description.clone());
        metrics.record_player_choice(choice.id.clone());

        // Generate branch if needed
        if choice.leads_to_step.is_some() {
            // Branch the narrative
            match self
                .quest_generator
                .branch_narrative(
                    &active_quest.quest,
                    &choice,
                    &self.create_context_from_quest(active_quest),
                )
                .await
            {
                Ok(branch) => {
                    info!("Generated quest branch: {}", branch.name);
                    // Apply branch consequences (would need more integration with game systems)
                    self.apply_choice_consequences(active_quest, &choice, &branch)
                        .await?;
                }
                Err(e) => {
                    error!("Failed to generate quest branch: {}", e);
                    // Continue with existing quest flow
                }
            }
        }

        // Advance quest step if appropriate
        if choice.leads_to_step.is_some() || self.choice_completes_step(&choice) {
            let tick = active_quest.last_update;
            active_quest.advance_step(tick);
        }

        Ok(())
    }

    /// Abandon a quest
    pub async fn abandon_quest(
        &self,
        active_quest: &mut CActiveQuest,
        reason: String,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
    ) -> Result<()> {
        active_quest.set_state(QuestState::Abandoned, active_quest.last_update);

        warn!("Quest '{}' abandoned: {}", active_quest.quest.title, reason);

        // Update metrics and journal
        metrics.record_quest_abandonment(&active_quest.quest, reason.clone());
        journal.abandon_quest(&active_quest.quest.id, reason);

        Ok(())
    }

    /// Force quest completion for testing or admin purposes
    pub async fn force_complete_quest(
        &self,
        active_quest: &mut CActiveQuest,
        metrics: &mut CQuestMetrics,
        journal: &mut CQuestJournal,
        completion_notes: String,
    ) -> Result<()> {
        active_quest.set_state(QuestState::Completed, active_quest.last_update);

        // Mark all steps as completed
        for step in &mut active_quest.quest.steps {
            step.completed = true;
        }

        info!(
            "Force completed quest '{}': {}",
            active_quest.quest.title, completion_notes
        );

        // Update metrics with admin completion
        let duration = active_quest.get_duration();
        metrics.record_quest_completion(&active_quest.quest, duration as f64, 1.0);
        journal.complete_quest(
            &active_quest.quest.id,
            format!("Force completed: {}", completion_notes),
        );

        Ok(())
    }

    /// Check if quest generation should occur
    fn should_generate_quest(
        &self,
        generator: &CQuestGenerator,
        journal: &CQuestJournal,
        current_time_ms: u64,
    ) -> bool {
        // Check cooldown
        if !generator.can_generate_quest(current_time_ms) {
            return false;
        }

        // Check active quest limits
        if !journal.can_accept_new_quest() {
            return false;
        }

        // Check if player has been active (has recent activities)
        if generator.context.recent_activities.is_empty() {
            return false;
        }

        true
    }

    /// Clean up completed/abandoned quests from active list
    fn cleanup_quests(
        &self,
        active_quests: &mut Vec<CActiveQuest>,
        _metrics: &mut CQuestMetrics,
        _journal: &mut CQuestJournal,
    ) {
        let initial_count = active_quests.len();

        active_quests
            .retain(|quest| matches!(quest.state, QuestState::Active | QuestState::Paused));

        let removed = initial_count - active_quests.len();
        if removed > 0 {
            debug!("Cleaned up {} completed/abandoned quests", removed);
        }
    }

    /// Generate dynamic content for a quest step
    async fn generate_dynamic_content_for_step(
        &self,
        active_quest: &CActiveQuest,
        step_id: String,
    ) -> Result<crate::llm_quests::DynamicContent> {
        let step = active_quest
            .quest
            .steps
            .iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| anyhow::anyhow!("Step not found: {}", step_id))?;

        let context = self.create_context_from_quest(active_quest);
        self.quest_generator
            .generate_dynamic_content(step, &context)
            .await
    }

    /// Create quest context from active quest
    fn create_context_from_quest(&self, active_quest: &CActiveQuest) -> QuestContext {
        QuestContext {
            player_id: active_quest.quest.personalization.player_id.clone(),
            player_level: active_quest.quest.metadata.player_level_range.0, // Use minimum for safety
            location: "current_location".to_string(), // Would come from game state
            available_npcs: Vec::new(),               // Would come from game state
            world_state: std::collections::HashMap::new(), // Would come from game state
            recent_activities: Vec::new(),            // Would come from game state
            preferred_quest_types: active_quest
                .quest
                .personalization
                .player_preferences
                .clone(),
        }
    }

    /// Apply consequences of player choice
    async fn apply_choice_consequences(
        &self,
        _active_quest: &mut CActiveQuest,
        _choice: &BranchingChoice,
        _branch: &crate::llm_quests::QuestBranch,
    ) -> Result<()> {
        // This would integrate with game systems to apply actual consequences
        // Examples: change world state, spawn items, modify NPC relationships
        debug!("Applying choice consequences (placeholder)");
        Ok(())
    }

    /// Check if choice completes the current step
    fn choice_completes_step(&self, choice: &BranchingChoice) -> bool {
        // Simple heuristic - could be more sophisticated
        choice
            .consequences
            .iter()
            .any(|c| c.contains("complete") || c.contains("finish"))
    }

    /// Calculate quality score for completed quest
    fn calculate_completion_quality(&self, active_quest: &CActiveQuest) -> f32 {
        let mut quality = 0.8; // Base quality

        // Bonus for player engagement (choices made)
        let choice_bonus = (active_quest.choices_made.len() as f32 * 0.05).min(0.2);
        quality += choice_bonus;

        // Penalty for very quick completion (might indicate lack of engagement)
        let duration_minutes = active_quest.get_duration() as i64;
        if duration_minutes < 5 {
            quality -= 0.3;
        } else if duration_minutes > active_quest.quest.metadata.estimated_duration as i64 * 3 {
            quality -= 0.1; // Slight penalty for taking much longer than expected
        }

        quality.clamp(0.1, 1.0)
    }
}

/// Helper functions for quest system integration
pub mod integration {
    use super::*;

    /// Initialize quest system for a player
    pub fn initialize_player_quest_system(
        player_id: String,
        player_level: u32,
        starting_location: String,
    ) -> (CQuestGenerator, CQuestJournal, CQuestMetrics) {
        let generator = CQuestGenerator::new(player_id, player_level, starting_location);
        let journal = CQuestJournal::new();
        let metrics = CQuestMetrics::default();

        (generator, journal, metrics)
    }

    /// Convert active quest to basic quest for compatibility
    pub fn to_basic_quest(active_quest: &CActiveQuest) -> crate::Quest {
        crate::Quest {
            title: active_quest.quest.title.clone(),
            steps: active_quest
                .quest
                .steps
                .iter()
                .map(|step| crate::QuestStep {
                    description: step.description.clone(),
                    completed: step.completed,
                })
                .collect(),
        }
    }

    /// Check if player should receive quest prompts
    pub fn should_prompt_for_quest(
        generator: &CQuestGenerator,
        journal: &CQuestJournal,
        current_time_ms: u64,
    ) -> bool {
        journal.auto_discover
            && journal.can_accept_new_quest()
            && generator.can_generate_quest(current_time_ms)
            && !generator.context.recent_activities.is_empty()
    }

    /// Get quest recommendations for player
    pub fn get_quest_recommendations(journal: &CQuestJournal) -> Vec<String> {
        let preferences = journal.get_preferred_categories();
        let stats = journal.get_statistics();

        let mut recommendations = Vec::new();

        if stats.completion_rate < 0.3 {
            recommendations
                .push("Consider shorter, simpler quests to build confidence".to_string());
        }

        if preferences.contains(&"exploration".to_string()) {
            recommendations.push("Exploration quests available in new areas".to_string());
        }

        if stats.active_quests == 0 {
            recommendations.push("No active quests - time for a new adventure!".to_string());
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_quests::{LlmQuestGenerator, QuestGenerationConfig};
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
    async fn test_quest_system_creation() {
        let llm_client: Arc<dyn astraweave_llm::LlmClient> = Arc::new(MockLlm);
        let rag_pipeline = create_test_rag_pipeline();
        let quest_generator = Arc::new(
            LlmQuestGenerator::new(llm_client, rag_pipeline, QuestGenerationConfig::default())
                .unwrap(),
        );

        let _system = QuestLlmSystem::new(quest_generator, 300000, 3);
        // Test basic system creation
    }

    #[test]
    fn test_player_quest_system_initialization() {
        let (generator, journal, metrics) = integration::initialize_player_quest_system(
            "test_player".to_string(),
            5,
            "forest".to_string(),
        );

        assert_eq!(generator.context.player_id, "test_player");
        assert_eq!(generator.context.player_level, 5);
        assert_eq!(generator.context.location, "forest");
        assert_eq!(journal.get_active_quest_count(), 0);
        assert_eq!(metrics.quests_generated, 0);
    }

    #[test]
    fn test_quest_recommendations() {
        let journal = CQuestJournal::new();
        let recommendations = integration::get_quest_recommendations(&journal);

        assert!(!recommendations.is_empty());
        assert!(recommendations
            .iter()
            .any(|r| r.contains("No active quests")));
    }
}
