//! Episode-based interaction recording for companion learning.
//!
//! Episodes represent temporal chunks of player-companion interaction
//! that are stored as Episodic memories in the existing memory system.
//!
//! This module provides structures for recording complete interaction
//! episodes (combat encounters, dialogue sessions, exploration periods)
//! that can be analyzed to detect patterns and drive behavioral learning.

use crate::{
    EmotionalContext, Memory, MemoryContent, MemoryMetadata, MemorySource, MemoryType,
    SpatialTemporalContext,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Episode category aligned with gameplay systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EpisodeCategory {
    /// Combat encounter episode
    Combat,
    /// Dialogue/conversation episode
    Dialogue,
    /// Exploration and discovery episode
    Exploration,
    /// Puzzle-solving episode
    Puzzle,
    /// Quest/objective progression episode
    Quest,
    /// Social interaction episode
    Social,
}

impl std::fmt::Display for EpisodeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpisodeCategory::Combat => write!(f, "Combat"),
            EpisodeCategory::Dialogue => write!(f, "Dialogue"),
            EpisodeCategory::Exploration => write!(f, "Exploration"),
            EpisodeCategory::Puzzle => write!(f, "Puzzle"),
            EpisodeCategory::Quest => write!(f, "Quest"),
            EpisodeCategory::Social => write!(f, "Social"),
        }
    }
}

/// Player action during an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerAction {
    /// Type of action performed (e.g., "melee_attack", "cast_spell", "use_item")
    pub action_type: String,
    /// Target of the action if applicable
    pub target: Option<String>,
    /// Action-specific parameters (damage, range, etc.)
    pub parameters: serde_json::Value,
}

/// Companion response to player action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionResponse {
    /// Type of response action
    pub action_type: String,
    /// Result of the companion's action
    pub result: ActionResult,
    /// How effective the response was (0.0 to 1.0)
    pub effectiveness: f32,
}

/// Result of an action attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[must_use]
pub enum ActionResult {
    /// Action completed successfully
    Success,
    /// Action failed to complete
    Failure,
    /// Action was interrupted
    Interrupted,
    /// Action partially succeeded
    Partial,
}

impl ActionResult {
    /// Get success multiplier for effectiveness calculations
    pub fn success_multiplier(&self) -> f32 {
        match self {
            ActionResult::Success => 1.0,
            ActionResult::Partial => 0.5,
            ActionResult::Interrupted => 0.25,
            ActionResult::Failure => 0.0,
        }
    }
}

/// Metrics tracking episode outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeOutcome {
    /// Overall success rating (0.0 = total failure, 1.0 = perfect success)
    pub success_rating: f32,
    /// Inferred player satisfaction (0.0 to 1.0)
    pub player_satisfaction: f32,
    /// How effectively companion contributed (0.0 to 1.0)
    pub companion_effectiveness: f32,
    /// Episode duration in milliseconds
    pub duration_ms: u64,
    /// Total damage dealt by player and companion
    pub damage_dealt: f32,
    /// Total damage taken by player and companion
    pub damage_taken: f32,
    /// Resources consumed (health, mana, items)
    pub resources_used: f32,
    /// Number of failed attempts or deaths
    pub failure_count: u32,
}

impl EpisodeOutcome {
    /// Calculate composite quality score
    pub fn quality_score(&self) -> f32 {
        let efficiency = if self.resources_used > 0.0 {
            (self.damage_dealt / self.resources_used).min(1.0)
        } else {
            1.0
        };

        let survivability = if self.damage_dealt + self.damage_taken > 0.0 {
            self.damage_dealt / (self.damage_dealt + self.damage_taken)
        } else {
            0.5
        };

        // Weighted average of all factors
        (self.success_rating * 0.4
            + self.player_satisfaction * 0.3
            + self.companion_effectiveness * 0.2
            + efficiency * 0.05
            + survivability * 0.05)
            .clamp(0.0, 1.0)
    }
}

/// Single observation during an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Timestamp relative to episode start (milliseconds)
    pub timestamp_ms: u64,
    /// Player action if any occurred
    pub player_action: Option<PlayerAction>,
    /// Companion response if any occurred
    pub companion_response: Option<CompanionResponse>,
    /// World state snapshot at this observation
    pub world_state: serde_json::Value,
}

impl Observation {
    /// Create new observation with current timestamp
    pub fn new(
        timestamp_ms: u64,
        player_action: Option<PlayerAction>,
        companion_response: Option<CompanionResponse>,
        world_state: serde_json::Value,
    ) -> Self {
        Self {
            timestamp_ms,
            player_action,
            companion_response,
            world_state,
        }
    }

    /// Extract player health from world state if present
    pub fn player_health(&self) -> Option<f32> {
        self.world_state
            .get("player_health")
            .and_then(|v| v.as_f64())
            .map(|h| h as f32)
    }

    /// Extract enemy count from world state if present
    pub fn enemy_count(&self) -> Option<u32> {
        self.world_state
            .get("enemy_count")
            .and_then(|v| v.as_u64())
            .map(|c| c as u32)
    }
}

/// Complete episode structure that converts to Memory for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique episode identifier
    pub id: String,
    /// Category of episode
    pub category: EpisodeCategory,
    /// Episode start time
    pub start_time: SystemTime,
    /// Episode end time (None if still active)
    pub end_time: Option<SystemTime>,
    /// Chronological observations during episode
    pub observations: Vec<Observation>,
    /// Outcome metrics (None if episode incomplete)
    pub outcome: Option<EpisodeOutcome>,
    /// Tags for categorization and retrieval
    pub tags: Vec<String>,
}

impl Episode {
    /// Create new episode with unique ID
    pub fn new(id: String, category: EpisodeCategory) -> Self {
        Self {
            id,
            category,
            start_time: SystemTime::now(),
            end_time: None,
            observations: Vec::new(),
            outcome: None,
            tags: Vec::new(),
        }
    }

    /// Add observation to episode
    pub fn add_observation(&mut self, observation: Observation) {
        self.observations.push(observation);
    }

    /// Complete episode with outcome
    pub fn complete(&mut self, outcome: EpisodeOutcome) {
        self.end_time = Some(SystemTime::now());
        self.outcome = Some(outcome);
    }

    /// Add tag to episode
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get episode duration
    pub fn duration(&self) -> Option<Duration> {
        self.end_time
            .and_then(|end| end.duration_since(self.start_time).ok())
    }

    /// Check if episode is complete
    pub fn is_complete(&self) -> bool {
        self.end_time.is_some() && self.outcome.is_some()
    }

    /// Convert episode to Episodic memory for storage
    ///
    /// This integration with existing memory system allows episodes
    /// to leverage existing retrieval, consolidation, and forgetting
    /// mechanisms.
    pub fn to_memory(&self) -> anyhow::Result<Memory> {
        // Build descriptive text from outcome
        let outcome_str = self
            .outcome
            .as_ref()
            .map(|o| {
                format!(
                    "Success: {:.0}%, Satisfaction: {:.0}%, Quality: {:.0}%",
                    o.success_rating * 100.0,
                    o.player_satisfaction * 100.0,
                    o.quality_score() * 100.0
                )
            })
            .unwrap_or_else(|| "In progress".to_string());

        // Extract location from first observation if available
        let location = self
            .observations
            .first()
            .and_then(|obs| obs.world_state.get("location"))
            .and_then(|loc| loc.as_str())
            .map(String::from);

        // Extract participants from observations
        let mut participants = vec!["player".to_string(), "companion".to_string()];
        if let Some(first_obs) = self.observations.first() {
            if let Some(enemy_count) = first_obs.enemy_count() {
                for i in 0..enemy_count {
                    participants.push(format!("enemy_{}", i));
                }
            }
        }

        // Create emotional context from outcome
        let emotional_context = self.outcome.as_ref().map(|o| {
            let primary_emotion = if o.success_rating > 0.8 {
                "triumphant"
            } else if o.success_rating > 0.6 {
                "satisfied"
            } else if o.success_rating > 0.4 {
                "uncertain"
            } else if o.success_rating > 0.2 {
                "frustrated"
            } else {
                "defeated"
            };

            EmotionalContext {
                primary_emotion: primary_emotion.to_string(),
                intensity: o.player_satisfaction,
                valence: o.success_rating * 2.0 - 1.0, // Map 0-1 to -1-1
                arousal: o.companion_effectiveness,
            }
        });

        // Create spatial-temporal context
        let context = SpatialTemporalContext {
            location,
            time_period: None,
            duration: self
                .duration()
                .map(|d| d.as_millis().try_into().unwrap_or(u64::MAX)),
            participants,
            related_events: vec![],
        };

        // Create memory content with episode data
        let content = MemoryContent {
            text: format!("{} episode: {}", self.category, outcome_str),
            data: serde_json::to_value(self)?,
            sensory_data: None,
            emotional_context,
            context,
        };

        // Calculate importance from outcome quality
        let importance = self
            .outcome
            .as_ref()
            .map(|o| o.quality_score())
            .unwrap_or(0.5);

        // Create memory metadata
        let metadata = MemoryMetadata {
            created_at: time::OffsetDateTime::now_utc(),
            last_accessed: time::OffsetDateTime::now_utc(),
            access_count: 0,
            importance,
            confidence: 1.0,
            source: MemorySource::DirectExperience,
            tags: self.tags.clone(),
            permanent: false,
            strength: 1.0,
            decay_factor: 1.0,
        };

        Ok(Memory {
            id: self.id.clone(),
            memory_type: MemoryType::Episodic,
            content,
            metadata,
            associations: vec![],
            embedding: None,
        })
    }

    /// Compute average player health during episode
    pub fn average_player_health(&self) -> Option<f32> {
        let healths: Vec<f32> = self
            .observations
            .iter()
            .filter_map(|obs| obs.player_health())
            .collect();

        if healths.is_empty() {
            None
        } else {
            Some(healths.iter().sum::<f32>() / healths.len() as f32)
        }
    }

    /// Count observations by action type
    pub fn count_actions(&self, action_type: &str) -> usize {
        self.observations
            .iter()
            .filter(|obs| {
                obs.player_action
                    .as_ref()
                    .map(|a| a.action_type.contains(action_type))
                    .unwrap_or(false)
            })
            .count()
    }

    /// Calculate action diversity (unique action types)
    pub fn action_diversity(&self) -> usize {
        use std::collections::HashSet;
        let mut unique_actions = HashSet::new();

        for obs in &self.observations {
            if let Some(action) = &obs.player_action {
                unique_actions.insert(&action.action_type);
            }
        }

        unique_actions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episode_creation() {
        let episode = Episode::new("test_123".to_string(), EpisodeCategory::Combat);

        assert_eq!(episode.id, "test_123");
        assert_eq!(episode.category, EpisodeCategory::Combat);
        assert!(!episode.is_complete());
        assert_eq!(episode.observations.len(), 0);
    }

    #[test]
    fn test_episode_completion() {
        let mut episode = Episode::new("test_456".to_string(), EpisodeCategory::Combat);

        let outcome = EpisodeOutcome {
            success_rating: 0.9,
            player_satisfaction: 0.85,
            companion_effectiveness: 0.8,
            duration_ms: 45_000,
            damage_dealt: 500.0,
            damage_taken: 100.0,
            resources_used: 150.0,
            failure_count: 0,
        };

        episode.complete(outcome);

        assert!(episode.is_complete());
        assert!(episode.duration().is_some());
    }

    #[test]
    fn test_episode_observations() {
        let mut episode = Episode::new("test_789".to_string(), EpisodeCategory::Combat);

        let obs = Observation::new(
            1000,
            Some(PlayerAction {
                action_type: "melee_attack".to_string(),
                target: Some("enemy".to_string()),
                parameters: serde_json::json!({"damage": 50}),
            }),
            Some(CompanionResponse {
                action_type: "defensive_stance".to_string(),
                result: ActionResult::Success,
                effectiveness: 0.9,
            }),
            serde_json::json!({"player_health": 0.8, "enemy_count": 2}),
        );

        episode.add_observation(obs);

        assert_eq!(episode.observations.len(), 1);
        assert_eq!(episode.observations[0].player_health(), Some(0.8));
        assert_eq!(episode.observations[0].enemy_count(), Some(2));
    }

    #[test]
    fn test_episode_to_memory_conversion() {
        let mut episode = Episode::new("mem_test".to_string(), EpisodeCategory::Combat);
        episode.add_tag("aggressive".to_string());
        episode.add_tag("melee".to_string());

        let outcome = EpisodeOutcome {
            success_rating: 0.95,
            player_satisfaction: 1.0,
            companion_effectiveness: 0.85,
            duration_ms: 30_000,
            damage_dealt: 300.0,
            damage_taken: 50.0,
            resources_used: 100.0,
            failure_count: 0,
        };

        episode.complete(outcome);

        let memory = episode.to_memory().expect("Failed to convert to memory");

        assert_eq!(memory.id, "mem_test");
        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert_eq!(memory.metadata.tags.len(), 2);
        assert!(memory.metadata.importance > 0.8); // High quality episode
        assert!(memory.content.text.contains("Combat episode"));
    }

    #[test]
    fn test_outcome_quality_score() {
        let outcome = EpisodeOutcome {
            success_rating: 0.9,
            player_satisfaction: 0.85,
            companion_effectiveness: 0.8,
            duration_ms: 45_000,
            damage_dealt: 500.0,
            damage_taken: 100.0,
            resources_used: 150.0,
            failure_count: 0,
        };

        let quality = outcome.quality_score();
        assert!(
            quality > 0.8,
            "Expected high quality score, got {}",
            quality
        );
    }

    #[test]
    fn test_action_counting() {
        let mut episode = Episode::new("action_test".to_string(), EpisodeCategory::Combat);

        for i in 0..3 {
            episode.add_observation(Observation::new(
                i * 1000,
                Some(PlayerAction {
                    action_type: "melee_attack".to_string(),
                    target: Some("enemy".to_string()),
                    parameters: serde_json::json!({}),
                }),
                None,
                serde_json::json!({}),
            ));
        }

        for i in 3..5 {
            episode.add_observation(Observation::new(
                i * 1000,
                Some(PlayerAction {
                    action_type: "ranged_attack".to_string(),
                    target: Some("enemy".to_string()),
                    parameters: serde_json::json!({}),
                }),
                None,
                serde_json::json!({}),
            ));
        }

        assert_eq!(episode.count_actions("melee"), 3);
        assert_eq!(episode.count_actions("ranged"), 2);
        assert_eq!(episode.action_diversity(), 2);
    }

    #[test]
    fn test_average_player_health() {
        let mut episode = Episode::new("health_test".to_string(), EpisodeCategory::Combat);

        let healths = [1.0, 0.8, 0.6, 0.4];
        for (i, health) in healths.iter().enumerate() {
            episode.add_observation(Observation::new(
                i as u64 * 1000,
                None,
                None,
                serde_json::json!({"player_health": health}),
            ));
        }

        let avg = episode
            .average_player_health()
            .expect("Should have average");
        assert!((avg - 0.7).abs() < 0.01, "Expected 0.7, got {}", avg);
    }
}
