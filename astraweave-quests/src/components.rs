use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::llm_quests::{
    BranchingChoice, DynamicContent, LlmQuest, QuestContext, QuestGenerationConfig, QuestValidation,
};

/// ECS component for quest generation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CQuestGenerator {
    /// Active quest context
    pub context: QuestContext,
    /// Configuration for quest generation
    pub config: QuestGenerationConfig,
    /// Currently active quests for this entity
    pub active_quests: Vec<String>, // Quest IDs
    /// Quest generation cooldown
    pub last_generation_time: u64,
    pub generation_cooldown_ms: u64,
}

impl Default for CQuestGenerator {
    fn default() -> Self {
        Self {
            context: QuestContext {
                player_id: "default".to_string(),
                player_level: 1,
                location: "starting_area".to_string(),
                available_npcs: Vec::new(),
                world_state: HashMap::new(),
                recent_activities: Vec::new(),
                preferred_quest_types: vec!["exploration".to_string()],
            },
            config: QuestGenerationConfig::default(),
            active_quests: Vec::new(),
            last_generation_time: 0,
            generation_cooldown_ms: 300000, // 5 minutes
        }
    }
}

impl CQuestGenerator {
    pub fn new(player_id: String, player_level: u32, location: String) -> Self {
        Self {
            context: QuestContext {
                player_id,
                player_level,
                location,
                available_npcs: Vec::new(),
                world_state: HashMap::new(),
                recent_activities: Vec::new(),
                preferred_quest_types: vec!["exploration".to_string()],
            },
            config: QuestGenerationConfig::default(),
            active_quests: Vec::new(),
            last_generation_time: 0,
            generation_cooldown_ms: 300000,
        }
    }

    /// Check if quest generation is ready (cooldown passed)
    pub fn can_generate_quest(&self, current_time_ms: u64) -> bool {
        current_time_ms - self.last_generation_time >= self.generation_cooldown_ms
    }

    /// Add a quest to active quests
    pub fn add_active_quest(&mut self, quest_id: String) {
        if !self.active_quests.contains(&quest_id) {
            self.active_quests.push(quest_id);
        }
    }

    /// Remove a quest from active quests
    pub fn remove_active_quest(&mut self, quest_id: &str) {
        self.active_quests.retain(|id| id != quest_id);
    }

    /// Update context with new information
    pub fn update_context(
        &mut self,
        location: Option<String>,
        available_npcs: Option<Vec<String>>,
        world_state: Option<HashMap<String, serde_json::Value>>,
    ) {
        if let Some(loc) = location {
            self.context.location = loc;
        }
        if let Some(npcs) = available_npcs {
            self.context.available_npcs = npcs;
        }
        if let Some(state) = world_state {
            self.context.world_state = state;
        }
    }

    /// Add recent activity
    pub fn add_recent_activity(&mut self, activity: String) {
        self.context.recent_activities.push(activity);

        // Keep only recent activities
        if self.context.recent_activities.len() > 10 {
            self.context.recent_activities.remove(0);
        }
    }

    /// Update last generation time
    pub fn mark_generation_time(&mut self, current_time_ms: u64) {
        self.last_generation_time = current_time_ms;
    }
}

/// ECS component for active quest tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CActiveQuest {
    /// The quest data
    pub quest: LlmQuest,
    /// Current step index
    pub current_step_index: usize,
    /// Sim tick when the quest started.
    pub start_time: u64,
    /// Sim tick of the last state change.
    pub last_update: u64,
    /// Quest state
    pub state: QuestState,
    /// Player choices made so far
    pub choices_made: Vec<ChoiceRecord>,
    /// Dynamic content cache
    pub dynamic_content: HashMap<String, DynamicContent>,
}

/// State of an active quest
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuestState {
    Active,
    Paused,
    Completed,
    Failed,
    Abandoned,
}

/// Record of a player choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceRecord {
    pub step_id: String,
    pub choice: BranchingChoice,
    pub timestamp: u64,
    pub consequences_applied: Vec<String>,
}

impl CActiveQuest {
    pub fn new(quest: LlmQuest) -> Self {
        Self::at_tick(quest, 0)
    }

    /// Start a quest on a sim tick. The tick is not a wall-clock reading.
    pub fn at_tick(quest: LlmQuest, tick: u64) -> Self {
        Self {
            quest,
            current_step_index: 0,
            start_time: tick,
            last_update: tick,
            state: QuestState::Active,
            choices_made: Vec::new(),
            dynamic_content: HashMap::new(),
        }
    }

    /// Get current quest step
    pub fn get_current_step(&self) -> Option<&crate::llm_quests::LlmQuestStep> {
        self.quest.steps.get(self.current_step_index)
    }

    /// Check if quest is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, QuestState::Completed) || self.quest.steps.iter().all(|s| s.completed)
    }

    /// Advance to next step
    pub fn advance_step(&mut self, tick: u64) -> bool {
        self.last_update = tick;
        if self.current_step_index + 1 < self.quest.steps.len() {
            self.current_step_index += 1;
            true
        } else {
            self.state = QuestState::Completed;
            false
        }
    }

    /// Record a player choice
    pub fn record_choice(&mut self, step_id: String, choice: BranchingChoice, tick: u64) {
        let record = ChoiceRecord {
            step_id,
            choice,
            timestamp: tick,
            consequences_applied: Vec::new(),
        };
        self.choices_made.push(record);
        self.last_update = tick;
    }

    /// Sim ticks since the quest started, using the last stamped tick.
    #[must_use]
    pub fn get_duration(&self) -> u64 {
        self.last_update.saturating_sub(self.start_time)
    }

    /// Update quest state
    pub fn set_state(&mut self, state: QuestState, tick: u64) {
        self.state = state;
        self.last_update = tick;
    }

    /// Add dynamic content for a step
    pub fn add_dynamic_content(&mut self, step_id: String, content: DynamicContent) {
        self.dynamic_content.insert(step_id, content);
    }

    /// Get dynamic content for a step
    pub fn get_dynamic_content(&self, step_id: &str) -> Option<&DynamicContent> {
        self.dynamic_content.get(step_id)
    }
}

/// ECS component for quest metrics and analytics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CQuestMetrics {
    /// Total quests generated
    pub quests_generated: u64,
    /// Total quests completed
    pub quests_completed: u64,
    /// Total quests abandoned
    pub quests_abandoned: u64,
    /// Average quest completion time
    pub average_completion_time: f64, // minutes
    /// Most popular quest categories
    pub category_popularity: HashMap<String, u32>,
    /// Branching choice statistics
    pub choice_statistics: HashMap<String, u32>,
    /// Quality scores of generated quests
    pub quality_scores: Vec<f32>,
    /// LLM generation metrics
    pub generation_metrics: GenerationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationMetrics {
    pub total_generations: u64,
    pub failed_generations: u64,
    pub average_generation_time: f32, // milliseconds
    pub validation_failures: u64,
    pub average_quality_score: f32,
}

impl CQuestMetrics {
    /// Record a completed quest
    pub fn record_quest_completion(
        &mut self,
        quest: &LlmQuest,
        completion_time_minutes: f64,
        quality_score: f32,
    ) {
        self.quests_completed += 1;

        // Update average completion time
        let total_time = self.average_completion_time * (self.quests_completed - 1) as f64
            + completion_time_minutes;
        self.average_completion_time = total_time / self.quests_completed as f64;

        // Track category popularity
        *self
            .category_popularity
            .entry(quest.metadata.category.clone())
            .or_insert(0) += 1;

        // Record quality score
        self.quality_scores.push(quality_score);
        if self.quality_scores.len() > 100 {
            self.quality_scores.remove(0);
        }
    }

    /// Record a quest abandonment
    pub fn record_quest_abandonment(&mut self, _quest: &LlmQuest, reason: String) {
        self.quests_abandoned += 1;

        // Track abandonment reasons (could add more sophisticated tracking)
        *self
            .choice_statistics
            .entry(format!("abandoned:{}", reason))
            .or_insert(0) += 1;
    }

    /// Record a quest generation
    pub fn record_quest_generation(
        &mut self,
        generation_time_ms: f32,
        success: bool,
        validation: Option<&QuestValidation>,
    ) {
        self.quests_generated += 1;
        self.generation_metrics.total_generations += 1;

        if !success {
            self.generation_metrics.failed_generations += 1;
        }

        // Update average generation time
        let total_time = self.generation_metrics.average_generation_time
            * (self.generation_metrics.total_generations - 1) as f32
            + generation_time_ms;
        self.generation_metrics.average_generation_time =
            total_time / self.generation_metrics.total_generations as f32;

        if let Some(validation) = validation {
            if !validation.is_valid {
                self.generation_metrics.validation_failures += 1;
            }

            // Update average quality score
            let quality_count = (self.generation_metrics.total_generations
                - self.generation_metrics.failed_generations)
                as f32;
            if quality_count > 0.0 {
                let total_quality = self.generation_metrics.average_quality_score
                    * (quality_count - 1.0)
                    + validation.quality_score;
                self.generation_metrics.average_quality_score = total_quality / quality_count;
            }
        }
    }

    /// Record a player choice
    pub fn record_player_choice(&mut self, choice_id: String) {
        *self.choice_statistics.entry(choice_id).or_insert(0) += 1;
    }

    /// Get completion rate
    pub fn get_completion_rate(&self) -> f32 {
        if self.quests_generated == 0 {
            0.0
        } else {
            self.quests_completed as f32 / self.quests_generated as f32
        }
    }

    /// Get abandonment rate
    pub fn get_abandonment_rate(&self) -> f32 {
        if self.quests_generated == 0 {
            0.0
        } else {
            self.quests_abandoned as f32 / self.quests_generated as f32
        }
    }

    /// Get average quality score
    pub fn get_average_quality(&self) -> f32 {
        if self.quality_scores.is_empty() {
            0.0
        } else {
            self.quality_scores.iter().sum::<f32>() / self.quality_scores.len() as f32
        }
    }

    /// Get generation success rate
    pub fn get_generation_success_rate(&self) -> f32 {
        if self.generation_metrics.total_generations == 0 {
            0.0
        } else {
            1.0 - (self.generation_metrics.failed_generations as f32
                / self.generation_metrics.total_generations as f32)
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// ECS component for quest journal/log functionality
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CQuestJournal {
    /// All quests (active and completed)
    pub quest_history: Vec<QuestJournalEntry>,
    /// Quick access to active quests
    pub active_quest_ids: Vec<String>,
    /// Player preferences learned from quest interactions
    pub learned_preferences: HashMap<String, f32>,
    /// Quest discovery settings
    pub auto_discover: bool,
    pub max_active_quests: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestJournalEntry {
    pub quest_id: String,
    pub quest_title: String,
    pub quest_description: String,
    pub category: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub abandoned_at: Option<u64>,
    pub final_state: QuestState,
    pub choices_made: Vec<String>, // Simplified choice descriptions
    pub completion_notes: String,
}

impl CQuestJournal {
    pub fn new() -> Self {
        Self {
            quest_history: Vec::new(),
            active_quest_ids: Vec::new(),
            learned_preferences: HashMap::new(),
            auto_discover: true,
            max_active_quests: 3,
        }
    }

    /// Add a new quest to the journal
    pub fn add_quest(&mut self, quest: &LlmQuest) {
        let entry = QuestJournalEntry {
            quest_id: quest.id.clone(),
            quest_title: quest.title.clone(),
            quest_description: quest.description.clone(),
            category: quest.metadata.category.clone(),
            started_at: quest.generated_at,
            completed_at: None,
            abandoned_at: None,
            final_state: QuestState::Active,
            choices_made: Vec::new(),
            completion_notes: String::new(),
        };

        self.quest_history.push(entry);
        self.active_quest_ids.push(quest.id.clone());

        // Update preferences based on quest acceptance
        *self
            .learned_preferences
            .entry(quest.metadata.category.clone())
            .or_insert(0.0) += 0.1;
    }

    /// Complete a quest in the journal
    pub fn complete_quest(&mut self, quest_id: &str, completion_notes: String) {
        self.complete_quest_at(quest_id, completion_notes, 0);
    }

    pub fn complete_quest_at(&mut self, quest_id: &str, completion_notes: String, tick: u64) {
        if let Some(entry) = self
            .quest_history
            .iter_mut()
            .find(|e| e.quest_id == quest_id)
        {
            entry.completed_at = Some(tick);
            entry.final_state = QuestState::Completed;
            entry.completion_notes = completion_notes;

            // Increase preference for completed quest type
            *self
                .learned_preferences
                .entry(entry.category.clone())
                .or_insert(0.0) += 0.2;
        }

        self.active_quest_ids.retain(|id| id != quest_id);
    }

    /// Abandon a quest in the journal
    pub fn abandon_quest(&mut self, quest_id: &str, reason: String) {
        self.abandon_quest_at(quest_id, reason, 0);
    }

    pub fn abandon_quest_at(&mut self, quest_id: &str, reason: String, tick: u64) {
        if let Some(entry) = self
            .quest_history
            .iter_mut()
            .find(|e| e.quest_id == quest_id)
        {
            entry.abandoned_at = Some(tick);
            entry.final_state = QuestState::Abandoned;
            entry.completion_notes = format!("Abandoned: {}", reason);

            // Decrease preference for abandoned quest type
            *self
                .learned_preferences
                .entry(entry.category.clone())
                .or_insert(0.0) -= 0.1;
        }

        self.active_quest_ids.retain(|id| id != quest_id);
    }

    /// Record a choice made in a quest
    pub fn record_choice(&mut self, quest_id: &str, choice_description: String) {
        if let Some(entry) = self
            .quest_history
            .iter_mut()
            .find(|e| e.quest_id == quest_id)
        {
            entry.choices_made.push(choice_description);
        }
    }

    /// Get active quest count
    pub fn get_active_quest_count(&self) -> usize {
        self.active_quest_ids.len()
    }

    /// Can accept new quest based on limits
    pub fn can_accept_new_quest(&self) -> bool {
        self.get_active_quest_count() < self.max_active_quests
    }

    /// Get preferred quest categories
    pub fn get_preferred_categories(&self) -> Vec<String> {
        let mut preferences: Vec<_> = self.learned_preferences.iter().collect();
        preferences.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
        preferences
            .into_iter()
            .take(3)
            .map(|(cat, _)| cat.clone())
            .collect()
    }

    /// Get quest statistics
    pub fn get_statistics(&self) -> QuestJournalStats {
        let completed = self
            .quest_history
            .iter()
            .filter(|e| matches!(e.final_state, QuestState::Completed))
            .count();
        let abandoned = self
            .quest_history
            .iter()
            .filter(|e| matches!(e.final_state, QuestState::Abandoned))
            .count();
        let total = self.quest_history.len();

        QuestJournalStats {
            total_quests: total,
            completed_quests: completed,
            abandoned_quests: abandoned,
            active_quests: self.active_quest_ids.len(),
            completion_rate: if total > 0 {
                completed as f32 / total as f32
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestJournalStats {
    pub total_quests: usize,
    pub completed_quests: usize,
    pub abandoned_quests: usize,
    pub active_quests: usize,
    pub completion_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_quests::{PersonalizationData, QuestBranching, QuestMetadata, QuestRewards};

    fn create_test_quest() -> LlmQuest {
        LlmQuest {
            id: "test_quest".to_string(),
            title: "Test Quest".to_string(),
            description: "A test quest".to_string(),
            steps: Vec::new(),
            metadata: QuestMetadata {
                category: "test".to_string(),
                difficulty_level: 0.5,
                estimated_duration: 30,
                player_level_range: (1, 5),
                required_skills: Vec::new(),
                tags: Vec::new(),
                generated_reasoning: "test".to_string(),
            },
            branching: QuestBranching {
                has_multiple_paths: false,
                branch_points: Vec::new(),
                convergence_points: Vec::new(),
            },
            rewards: QuestRewards {
                experience: 100,
                currency: 10,
                items: Vec::new(),
                reputation_changes: HashMap::new(),
                unlock_content: Vec::new(),
            },
            generated_at: 0,
            personalization: PersonalizationData {
                player_id: "test_player".to_string(),
                player_preferences: Vec::new(),
                play_style: "test".to_string(),
                previous_choices: Vec::new(),
                difficulty_preference: 0.5,
            },
        }
    }

    #[test]
    fn test_quest_generator_component() {
        let mut generator = CQuestGenerator::new("player1".to_string(), 5, "forest".to_string());

        assert_eq!(generator.context.player_id, "player1");
        assert_eq!(generator.context.player_level, 5);
        assert_eq!(generator.context.location, "forest");

        generator.add_active_quest("quest1".to_string());
        assert_eq!(generator.active_quests.len(), 1);

        generator.remove_active_quest("quest1");
        assert_eq!(generator.active_quests.len(), 0);
    }

    #[test]
    fn test_active_quest_component() {
        let quest = create_test_quest();
        let mut active_quest = CActiveQuest::new(quest);

        assert_eq!(active_quest.current_step_index, 0);
        assert!(matches!(active_quest.state, QuestState::Active));

        active_quest.set_state(QuestState::Completed, active_quest.last_update);
        assert!(matches!(active_quest.state, QuestState::Completed));
    }

    #[test]
    fn test_quest_metrics() {
        let mut metrics = CQuestMetrics::default();
        let quest = create_test_quest();

        metrics.record_quest_completion(&quest, 30.0, 0.8);
        assert_eq!(metrics.quests_completed, 1);
        assert_eq!(metrics.average_completion_time, 30.0);

        metrics.record_quest_generation(100.0, true, None);
        assert_eq!(metrics.generation_metrics.total_generations, 1);
        assert_eq!(metrics.generation_metrics.average_generation_time, 100.0);
    }

    #[test]
    fn test_quest_journal() {
        let mut journal = CQuestJournal::new();
        let quest = create_test_quest();

        journal.add_quest(&quest);
        assert_eq!(journal.get_active_quest_count(), 1);

        journal.complete_quest(&quest.id, "Great quest!".to_string());
        assert_eq!(journal.get_active_quest_count(), 0);

        let stats = journal.get_statistics();
        assert_eq!(stats.completed_quests, 1);
        assert_eq!(stats.completion_rate, 1.0);
    }
}
