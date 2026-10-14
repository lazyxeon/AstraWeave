use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Core memory structure representing different types of memories in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: MemoryContent,
    pub metadata: MemoryMetadata,
    pub associations: Vec<MemoryAssociation>,
    pub embedding: Option<Vec<f32>>,
}

/// Types of memories in the hierarchical memory system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum MemoryType {
    /// Short-term sensory impressions
    Sensory,
    /// Active working memory for current tasks
    #[default]
    Working,
    /// Episodic memories of specific events
    Episodic,
    /// Semantic knowledge and facts
    Semantic,
    /// Procedural knowledge (skills, habits)
    Procedural,
    /// Emotional memories with affective content
    Emotional,
    /// Social memories about relationships and interactions
    Social,
}

/// Content of a memory with structured data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContent {
    /// Primary text content
    pub text: String,
    /// Structured data associated with the memory
    pub data: serde_json::Value,
    /// Sensory information if applicable
    pub sensory_data: Option<SensoryData>,
    /// Emotional context
    pub emotional_context: Option<EmotionalContext>,
    /// Spatial/temporal context
    pub context: SpatialTemporalContext,
}

/// Sensory information associated with memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryData {
    pub visual: Option<String>,
    pub auditory: Option<String>,
    pub tactile: Option<String>,
    pub environmental: Option<String>,
}

/// Emotional context of a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalContext {
    pub primary_emotion: String,
    pub intensity: f32, // 0.0 to 1.0
    pub valence: f32,   // -1.0 (negative) to 1.0 (positive)
    pub arousal: f32,   // 0.0 (calm) to 1.0 (excited)
}

/// Spatial and temporal context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialTemporalContext {
    pub location: Option<String>,
    pub time_period: Option<String>,
    pub duration: Option<u64>, // milliseconds
    pub participants: Vec<String>,
    pub related_events: Vec<String>,
}

/// Metadata about a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// When the memory was created
    pub created_at: DateTime<Utc>,
    /// When the memory was last accessed
    pub last_accessed: DateTime<Utc>,
    /// How many times the memory has been accessed
    pub access_count: u32,
    /// Importance score (0.0 to 1.0)
    pub importance: f32,
    /// Confidence in the memory's accuracy
    pub confidence: f32,
    /// Source of the memory (experience, learning, etc.)
    pub source: MemorySource,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Whether the memory can be forgotten
    pub permanent: bool,
    /// Strength of the memory (affects retrieval probability)
    pub strength: f32,
    /// Age-related decay factor
    pub decay_factor: f32,
}

/// Source of a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MemorySource {
    DirectExperience,
    Conversation,
    Learning,
    Inference,
    SharedMemory,
    SystemGenerated,
}

/// Association between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAssociation {
    /// ID of the associated memory
    pub memory_id: String,
    /// Type of association
    pub association_type: AssociationType,
    /// Strength of the association (0.0 to 1.0)
    pub strength: f32,
    /// When this association was formed
    pub formed_at: DateTime<Utc>,
}

/// Types of associations between memories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AssociationType {
    /// Temporal - happened at the same time
    Temporal,
    /// Spatial - happened in the same place
    Spatial,
    /// Causal - one caused the other
    Causal,
    /// Conceptual - related concepts
    Conceptual,
    /// Emotional - similar emotional content
    Emotional,
    /// Sequential - one followed the other
    Sequential,
    /// Contrast - opposite or contrasting
    Contrast,
}

/// Memory cluster grouping related memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCluster {
    pub id: String,
    pub name: String,
    pub cluster_type: ClusterType,
    pub memory_ids: Vec<String>,
    pub central_concept: String,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Types of memory clusters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClusterType {
    Event,        // Memories of a specific event
    Person,       // Memories related to a person
    Location,     // Memories tied to a location
    Concept,      // Memories about a concept
    Skill,        // Memories forming a skill
    Relationship, // Memories about a relationship
}

/// Memory consolidation state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationState {
    /// Level of consolidation (0.0 = not consolidated, 1.0 = fully consolidated)
    pub consolidation_level: f32,
    /// Number of consolidation passes
    pub passes: u32,
    /// Last consolidation time
    pub last_consolidation: DateTime<Utc>,
    /// Whether consolidation is needed
    pub needs_consolidation: bool,
    /// Priority for consolidation
    pub priority: f32,
}

/// Forgetting curve parameters for a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingCurve {
    /// Initial strength of the memory
    pub initial_strength: f32,
    /// Decay rate constant
    pub decay_rate: f32,
    /// Half-life of the memory in days
    pub half_life: f32,
    /// Minimum strength threshold for retention
    pub retention_threshold: f32,
    /// Whether the memory is immune to forgetting
    pub immune: bool,
}

/// Memory retrieval context for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalContext {
    /// Query text
    pub query: String,
    /// Current emotional state
    pub emotional_state: Option<EmotionalContext>,
    /// Current location
    pub location: Option<String>,
    /// Recent memories to consider
    pub recent_memory_ids: Vec<String>,
    /// Preferred memory types
    pub preferred_types: Vec<MemoryType>,
    /// Time window for temporal filtering
    pub time_window: Option<TimeWindow>,
    /// Maximum number of memories to retrieve
    pub limit: usize,
}

/// Time window for memory filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Memory sharing permissions and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingMetadata {
    /// Whether this memory can be shared
    pub shareable: bool,
    /// List of agents/entities that can access this memory
    pub authorized_entities: Vec<String>,
    /// Type of sharing allowed
    pub sharing_type: SharingType,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Conditions for sharing
    pub sharing_conditions: Vec<String>,
}

/// Types of memory sharing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SharingType {
    /// Full memory content can be shared
    Full,
    /// Only summary can be shared
    Summary,
    /// Only existence/metadata can be shared
    Metadata,
    /// Cannot be shared
    Restricted,
}

/// Privacy levels for memories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PrivacyLevel {
    Public,   // Can be shared with anyone
    Group,    // Can be shared within group
    Personal, // Only for this entity
    Secret,   // Highly protected
}

fn sim_time(tick: u64) -> DateTime<Utc> {
    let secs = i64::try_from(tick).unwrap_or(0);
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
}

impl Memory {
    /// Create a new memory
    pub fn new(memory_type: MemoryType, content: MemoryContent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            memory_type,
            content,
            metadata: MemoryMetadata {
                created_at: Utc::now(),
                last_accessed: Utc::now(),
                access_count: 0,
                importance: 0.5,
                confidence: 1.0,
                source: MemorySource::DirectExperience,
                tags: Vec::new(),
                permanent: false,
                strength: 1.0,
                decay_factor: 1.0,
            },
            associations: Vec::new(),
            embedding: None,
        }
    }

    /// Create a sensory memory
    pub fn sensory(text: String, sensory_data: Option<SensoryData>) -> Self {
        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Sensory, content);
        memory.metadata.importance = 0.2; // Sensory memories start with low importance
        memory.metadata.decay_factor = 2.0; // Decay faster
        memory
    }

    /// Create an episodic memory
    pub fn episodic(text: String, participants: Vec<String>, location: Option<String>) -> Self {
        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location,
                time_period: Some("current".to_string()),
                duration: None,
                participants,
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Episodic, content);
        memory.metadata.importance = 0.7; // Episodic memories are more important
        memory
    }

    /// Episodic memory stamped with a sim tick.
    ///
    /// Does not draw a UUID or read the wall clock. `id` is the caller's
    /// stable key. `tick` is seconds since the unix epoch.
    pub fn episodic_at_tick(
        id: impl Into<String>,
        text: String,
        participants: Vec<String>,
        location: Option<String>,
        tick: u64,
    ) -> Self {
        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location,
                time_period: Some("sim".to_string()),
                duration: None,
                participants,
                related_events: Vec::new(),
            },
        };
        let time = sim_time(tick);
        Self {
            id: id.into(),
            memory_type: MemoryType::Episodic,
            content,
            metadata: MemoryMetadata {
                created_at: time,
                last_accessed: time,
                access_count: 0,
                importance: 0.5,
                confidence: 1.0,
                source: MemorySource::DirectExperience,
                tags: Vec::new(),
                permanent: false,
                strength: 1.0,
                decay_factor: 1.0,
            },
            associations: Vec::new(),
            embedding: None,
        }
    }

    /// Create a semantic memory
    pub fn semantic(text: String, concept: String) -> Self {
        let mut data = HashMap::new();
        data.insert("concept", serde_json::json!(concept));

        let content = MemoryContent {
            text,
            data: serde_json::to_value(data).unwrap_or_default(),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Semantic, content);
        memory.metadata.importance = 0.8; // Semantic memories are very important
        memory.metadata.permanent = true; // Facts should be permanent
        memory
    }

    /// Create a working memory
    pub fn working(text: String) -> Self {
        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: Some("current".to_string()),
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Working, content);
        memory.metadata.importance = 0.6;
        memory
    }

    /// Create a procedural memory
    pub fn procedural(text: String, skill: String) -> Self {
        let mut data = HashMap::new();
        data.insert("skill", serde_json::json!(skill));

        let content = MemoryContent {
            text,
            data: serde_json::to_value(data).unwrap_or_default(),
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Procedural, content);
        memory.metadata.importance = 0.7;
        memory
    }

    /// Create an emotional memory
    pub fn emotional(text: String, emotion: String, intensity: f32) -> Self {
        let emotional_context = EmotionalContext {
            primary_emotion: emotion,
            intensity,
            valence: 0.0,
            arousal: intensity,
        };

        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data: None,
            emotional_context: Some(emotional_context),
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Emotional, content);
        memory.metadata.importance = 0.8;
        memory
    }

    /// Create a social memory
    pub fn social(text: String, participants: Vec<String>) -> Self {
        let content = MemoryContent {
            text,
            data: serde_json::Value::Null,
            sensory_data: None,
            emotional_context: None,
            context: SpatialTemporalContext {
                location: None,
                time_period: None,
                duration: None,
                participants,
                related_events: Vec::new(),
            },
        };

        let mut memory = Self::new(MemoryType::Social, content);
        memory.metadata.importance = 0.75;
        memory
    }

    /// Update access information
    pub fn accessed(&mut self) {
        self.metadata.last_accessed = Utc::now();
        self.metadata.access_count += 1;

        // Strengthen memory through access (spaced repetition effect)
        self.metadata.strength = (self.metadata.strength + 0.1).min(1.0);
    }

    /// Check if memory should be forgotten based on current strength
    pub fn should_forget(&self, threshold: f32) -> bool {
        if self.metadata.permanent {
            return false;
        }

        let current_strength = self.calculate_current_strength();
        current_strength < threshold
    }

    /// Calculate current memory strength based on age and access
    pub fn calculate_current_strength(&self) -> f32 {
        let age_days = (Utc::now() - self.metadata.created_at).num_days() as f32;
        let time_since_access = (Utc::now() - self.metadata.last_accessed).num_days() as f32;

        // Apply forgetting curve: strength = initial * e^(-decay * time)
        let base_strength =
            self.metadata.strength * (-0.1 * age_days * self.metadata.decay_factor).exp();

        // Boost from recent access
        let access_boost = if time_since_access < 1.0 { 0.2 } else { 0.0 };

        (base_strength + access_boost).clamp(0.0, 1.0)
    }

    /// Add an association to another memory
    pub fn add_association(
        &mut self,
        memory_id: String,
        association_type: AssociationType,
        strength: f32,
    ) {
        let association = MemoryAssociation {
            memory_id,
            association_type,
            strength: strength.clamp(0.0, 1.0),
            formed_at: Utc::now(),
        };

        self.associations.push(association);
    }

    /// Get memories strongly associated with this one
    pub fn get_strong_associations(&self, min_strength: f32) -> Vec<&MemoryAssociation> {
        self.associations
            .iter()
            .filter(|assoc| assoc.strength >= min_strength)
            .collect()
    }

    /// Check if memory matches retrieval context
    pub fn matches_context(&self, context: &RetrievalContext) -> bool {
        // Check memory type preference
        if !context.preferred_types.is_empty()
            && !context.preferred_types.contains(&self.memory_type)
        {
            return false;
        }

        // Check time window
        if let Some(window) = &context.time_window {
            if self.metadata.created_at < window.start || self.metadata.created_at > window.end {
                return false;
            }
        }

        // Check location
        if let Some(query_location) = &context.location {
            if let Some(memory_location) = &self.content.context.location {
                if memory_location != query_location {
                    return false;
                }
            }
        }

        true
    }

    /// Calculate relevance score for a query
    pub fn calculate_relevance(&self, context: &RetrievalContext) -> f32 {
        let mut relevance = 0.0;

        // Text similarity (simplified - would use embeddings in practice)
        let query_words: Vec<&str> = context.query.split_whitespace().collect();
        let memory_words: Vec<&str> = self.content.text.split_whitespace().collect();

        let common_words = query_words
            .iter()
            .filter(|word| memory_words.contains(word))
            .count();

        let text_similarity = if !query_words.is_empty() {
            common_words as f32 / query_words.len() as f32
        } else {
            0.0
        };

        relevance += text_similarity * 0.4;

        // Importance and strength
        relevance += self.metadata.importance * 0.3;
        relevance += self.calculate_current_strength() * 0.2;

        // Recency bonus
        let age_days = (Utc::now() - self.metadata.created_at).num_days() as f32;
        let recency_bonus = if age_days < 7.0 {
            0.1 * (7.0 - age_days) / 7.0
        } else {
            0.0
        };
        relevance += recency_bonus;

        relevance.min(1.0)
    }
}

impl MemoryCluster {
    pub fn new(name: String, cluster_type: ClusterType, central_concept: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            cluster_type,
            memory_ids: Vec::new(),
            central_concept,
            importance: 0.5,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Add a memory to this cluster
    pub fn add_memory(&mut self, memory_id: String) {
        if !self.memory_ids.contains(&memory_id) {
            self.memory_ids.push(memory_id);
            self.last_updated = Utc::now();
        }
    }

    /// Remove a memory from this cluster
    pub fn remove_memory(&mut self, memory_id: &str) {
        self.memory_ids.retain(|id| id != memory_id);
        self.last_updated = Utc::now();
    }

    /// Calculate cluster importance based on member memories
    pub fn calculate_importance(&self, memories: &[&Memory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }

        let total_importance: f32 = memories.iter().map(|m| m.metadata.importance).sum();
        let avg_importance = total_importance / memories.len() as f32;

        // Boost importance for larger clusters
        let size_bonus = (memories.len() as f32 / 10.0).min(0.2);

        (avg_importance + size_bonus).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // MEMORY TYPE TESTS
    // ========================================================================

    #[test]
    fn test_memory_type_default() {
        let default_type = MemoryType::default();
        assert_eq!(default_type, MemoryType::Working);
    }

    #[test]
    fn test_memory_type_equality() {
        assert_eq!(MemoryType::Sensory, MemoryType::Sensory);
        assert_eq!(MemoryType::Working, MemoryType::Working);
        assert_eq!(MemoryType::Episodic, MemoryType::Episodic);
        assert_eq!(MemoryType::Semantic, MemoryType::Semantic);
        assert_eq!(MemoryType::Procedural, MemoryType::Procedural);
        assert_eq!(MemoryType::Emotional, MemoryType::Emotional);
        assert_eq!(MemoryType::Social, MemoryType::Social);

        assert_ne!(MemoryType::Sensory, MemoryType::Working);
    }

    // ========================================================================
    // MEMORY FACTORY TESTS
    // ========================================================================

    #[test]
    fn test_memory_creation() {
        let memory = Memory::sensory("I see a red apple".to_string(), None);
        assert_eq!(memory.memory_type, MemoryType::Sensory);
        assert_eq!(memory.content.text, "I see a red apple");
        assert!(memory.metadata.importance < 0.5);
    }

    #[test]
    fn test_sensory_memory_creation() {
        let memory = Memory::sensory("I see a red apple".to_string(), None);

        assert_eq!(memory.memory_type, MemoryType::Sensory);
        assert_eq!(memory.metadata.importance, 0.2);
        assert_eq!(memory.metadata.decay_factor, 2.0);
        assert!(!memory.metadata.permanent);
    }

    #[test]
    fn test_sensory_memory_with_data() {
        let sensory = SensoryData {
            visual: Some("red apple".to_string()),
            auditory: Some("crunching sound".to_string()),
            tactile: Some("smooth".to_string()),
            environmental: Some("kitchen".to_string()),
        };

        let memory = Memory::sensory("Apple experience".to_string(), Some(sensory.clone()));

        assert!(memory.content.sensory_data.is_some());
        let data = memory.content.sensory_data.unwrap();
        assert_eq!(data.visual, Some("red apple".to_string()));
        assert_eq!(data.auditory, Some("crunching sound".to_string()));
    }

    #[test]
    fn test_episodic_memory_creation() {
        let participants = vec!["Alice".to_string(), "Bob".to_string()];
        let memory = Memory::episodic(
            "Met at cafe".to_string(),
            participants.clone(),
            Some("cafe".to_string()),
        );

        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert_eq!(memory.metadata.importance, 0.7);
        assert_eq!(memory.content.context.participants, participants);
        assert_eq!(memory.content.context.location, Some("cafe".to_string()));
        assert_eq!(
            memory.content.context.time_period,
            Some("current".to_string())
        );
    }

    #[test]
    fn test_semantic_memory_creation() {
        let memory = Memory::semantic("Water boils at 100C".to_string(), "temperature".to_string());

        assert_eq!(memory.memory_type, MemoryType::Semantic);
        assert_eq!(memory.metadata.importance, 0.8);
        assert!(memory.metadata.permanent);
        assert!(memory.content.data.is_object());
    }

    #[test]
    fn test_working_memory_creation() {
        let memory = Memory::working("Current task".to_string());

        assert_eq!(memory.memory_type, MemoryType::Working);
        assert_eq!(memory.metadata.importance, 0.6);
        assert_eq!(
            memory.content.context.time_period,
            Some("current".to_string())
        );
    }

    #[test]
    fn test_procedural_memory_creation() {
        let memory = Memory::procedural("Tying shoelaces".to_string(), "motor".to_string());

        assert_eq!(memory.memory_type, MemoryType::Procedural);
        assert_eq!(memory.metadata.importance, 0.7);
        assert!(memory.content.data.is_object());
    }

    #[test]
    fn test_emotional_memory_creation() {
        let memory = Memory::emotional("Lost my pet".to_string(), "sadness".to_string(), 0.9);

        assert_eq!(memory.memory_type, MemoryType::Emotional);
        assert_eq!(memory.metadata.importance, 0.8);

        let emotional = memory.content.emotional_context.unwrap();
        assert_eq!(emotional.primary_emotion, "sadness");
        assert_eq!(emotional.intensity, 0.9);
        assert_eq!(emotional.arousal, 0.9);
    }

    #[test]
    fn test_social_memory_creation() {
        let participants = vec!["John".to_string(), "Jane".to_string()];
        let memory = Memory::social("Party together".to_string(), participants.clone());

        assert_eq!(memory.memory_type, MemoryType::Social);
        assert_eq!(memory.metadata.importance, 0.75);
        assert_eq!(memory.content.context.participants, participants);
    }

    // ========================================================================
    // MEMORY STRENGTH AND DECAY TESTS
    // ========================================================================

    #[test]
    fn test_memory_strength_calculation() {
        let mut memory = Memory::sensory("Test memory".to_string(), None);
        let initial_strength = memory.calculate_current_strength();

        // Simulate aging
        memory.metadata.created_at = Utc::now() - chrono::Duration::days(10);
        let aged_strength = memory.calculate_current_strength();

        assert!(aged_strength < initial_strength);
    }

    #[test]
    fn test_memory_strength_with_recent_access() {
        let mut memory = Memory::working("Test".to_string());
        memory.metadata.created_at = Utc::now() - chrono::Duration::days(5);
        memory.metadata.last_accessed = Utc::now();

        let strength = memory.calculate_current_strength();

        // Recent access should boost strength
        assert!(strength > 0.0);
    }

    #[test]
    fn test_should_forget_permanent() {
        let mut memory = Memory::semantic("Fact".to_string(), "science".to_string());
        memory.metadata.permanent = true;
        memory.metadata.strength = 0.0; // Even with zero strength

        assert!(!memory.should_forget(0.5));
    }

    #[test]
    fn test_should_forget_weak_memory() {
        let mut memory = Memory::sensory("Faint impression".to_string(), None);
        memory.metadata.strength = 0.1;
        memory.metadata.created_at = Utc::now() - chrono::Duration::days(100);
        memory.metadata.last_accessed = Utc::now() - chrono::Duration::days(100);

        // With high decay and old age, should be below threshold
        assert!(memory.should_forget(0.5));
    }

    #[test]
    fn test_should_forget_strong_memory() {
        let memory = Memory::semantic("Important fact".to_string(), "knowledge".to_string());

        // Recent, important memory should not be forgotten
        assert!(!memory.should_forget(0.3));
    }

    #[test]
    fn test_accessed_updates_metadata() {
        let mut memory = Memory::working("Task".to_string());
        let initial_count = memory.metadata.access_count;
        let _initial_strength = memory.metadata.strength;

        // Set strength below 1.0 so we can observe the boost
        memory.metadata.strength = 0.5;
        let pre_access_strength = memory.metadata.strength;

        memory.accessed();

        assert_eq!(memory.metadata.access_count, initial_count + 1);
        assert!(memory.metadata.strength > pre_access_strength);
        assert!(memory.metadata.strength <= 1.0);
    }

    #[test]
    fn test_accessed_strength_caps_at_one() {
        let mut memory = Memory::working("Task".to_string());
        memory.metadata.strength = 0.95;

        memory.accessed();
        memory.accessed();
        memory.accessed();

        assert_eq!(memory.metadata.strength, 1.0);
    }

    // ========================================================================
    // MEMORY ASSOCIATION TESTS
    // ========================================================================

    #[test]
    fn test_memory_associations() {
        let mut memory = Memory::episodic(
            "Met John at the market".to_string(),
            vec!["John".to_string()],
            Some("market".to_string()),
        );

        memory.add_association(
            "other_memory_id".to_string(),
            AssociationType::Temporal,
            0.8,
        );

        assert_eq!(memory.associations.len(), 1);
        assert_eq!(
            memory.associations[0].association_type,
            AssociationType::Temporal
        );
    }

    #[test]
    fn test_add_association_clamps_strength() {
        let mut memory = Memory::working("Test".to_string());

        memory.add_association("m1".to_string(), AssociationType::Causal, 1.5);
        assert_eq!(memory.associations[0].strength, 1.0);

        memory.add_association("m2".to_string(), AssociationType::Causal, -0.5);
        assert_eq!(memory.associations[1].strength, 0.0);
    }

    #[test]
    fn test_get_strong_associations() {
        let mut memory = Memory::working("Test".to_string());

        memory.add_association("m1".to_string(), AssociationType::Temporal, 0.9);
        memory.add_association("m2".to_string(), AssociationType::Causal, 0.5);
        memory.add_association("m3".to_string(), AssociationType::Conceptual, 0.8);

        let strong = memory.get_strong_associations(0.7);
        assert_eq!(strong.len(), 2);
    }

    #[test]
    fn test_get_strong_associations_empty() {
        let mut memory = Memory::working("Test".to_string());
        memory.add_association("m1".to_string(), AssociationType::Temporal, 0.3);

        let strong = memory.get_strong_associations(0.5);
        assert!(strong.is_empty());
    }

    #[test]
    fn test_association_type_equality() {
        assert_eq!(AssociationType::Temporal, AssociationType::Temporal);
        assert_eq!(AssociationType::Spatial, AssociationType::Spatial);
        assert_eq!(AssociationType::Causal, AssociationType::Causal);
        assert_eq!(AssociationType::Conceptual, AssociationType::Conceptual);
        assert_eq!(AssociationType::Emotional, AssociationType::Emotional);
        assert_eq!(AssociationType::Sequential, AssociationType::Sequential);
        assert_eq!(AssociationType::Contrast, AssociationType::Contrast);

        assert_ne!(AssociationType::Temporal, AssociationType::Spatial);
    }

    // ========================================================================
    // RETRIEVAL CONTEXT TESTS
    // ========================================================================

    #[test]
    fn test_retrieval_context_matching() {
        let memory = Memory::episodic(
            "Visited the library".to_string(),
            vec!["librarian".to_string()],
            Some("library".to_string()),
        );

        let context = RetrievalContext {
            query: "library".to_string(),
            emotional_state: None,
            location: Some("library".to_string()),
            recent_memory_ids: Vec::new(),
            preferred_types: vec![MemoryType::Episodic],
            time_window: None,
            limit: 10,
        };

        assert!(memory.matches_context(&context));

        let relevance = memory.calculate_relevance(&context);
        assert!(relevance > 0.0);
    }

    #[test]
    fn test_matches_context_wrong_type() {
        let memory = Memory::sensory("Sound".to_string(), None);

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![MemoryType::Episodic],
            time_window: None,
            limit: 10,
        };

        assert!(!memory.matches_context(&context));
    }

    #[test]
    fn test_matches_context_empty_preferred_types() {
        let memory = Memory::sensory("Sound".to_string(), None);

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        assert!(memory.matches_context(&context));
    }

    #[test]
    fn test_matches_context_time_window() {
        let mut memory = Memory::working("Test".to_string());
        memory.metadata.created_at = Utc::now() - chrono::Duration::days(5);

        let context_in_window = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: Some(TimeWindow {
                start: Utc::now() - chrono::Duration::days(10),
                end: Utc::now(),
            }),
            limit: 10,
        };

        assert!(memory.matches_context(&context_in_window));

        let context_outside = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: Some(TimeWindow {
                start: Utc::now() - chrono::Duration::days(3),
                end: Utc::now(),
            }),
            limit: 10,
        };

        assert!(!memory.matches_context(&context_outside));
    }

    #[test]
    fn test_matches_context_location_mismatch() {
        let memory = Memory::episodic("Test".to_string(), vec![], Some("library".to_string()));

        let context = RetrievalContext {
            query: "test".to_string(),
            emotional_state: None,
            location: Some("park".to_string()),
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        assert!(!memory.matches_context(&context));
    }

    #[test]
    fn test_calculate_relevance_empty_query() {
        let memory = Memory::working("Some content".to_string());

        let context = RetrievalContext {
            query: "".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let relevance = memory.calculate_relevance(&context);
        // Should still have relevance from importance and strength
        assert!(relevance > 0.0);
    }

    #[test]
    fn test_calculate_relevance_matching_words() {
        let memory = Memory::working("apple banana cherry".to_string());

        let context = RetrievalContext {
            query: "apple banana".to_string(),
            emotional_state: None,
            location: None,
            recent_memory_ids: Vec::new(),
            preferred_types: vec![],
            time_window: None,
            limit: 10,
        };

        let relevance = memory.calculate_relevance(&context);
        assert!(relevance > 0.4); // High text similarity
    }

    // ========================================================================
    // MEMORY CLUSTER TESTS
    // ========================================================================

    #[test]
    fn test_cluster_management() {
        let mut cluster = MemoryCluster::new(
            "John Memories".to_string(),
            ClusterType::Person,
            "John".to_string(),
        );

        cluster.add_memory("memory1".to_string());
        cluster.add_memory("memory2".to_string());

        assert_eq!(cluster.memory_ids.len(), 2);

        cluster.remove_memory("memory1");
        assert_eq!(cluster.memory_ids.len(), 1);
    }

    #[test]
    fn test_cluster_add_duplicate() {
        let mut cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Event, "event".to_string());

        cluster.add_memory("m1".to_string());
        cluster.add_memory("m1".to_string());

        assert_eq!(cluster.memory_ids.len(), 1);
    }

    #[test]
    fn test_cluster_remove_nonexistent() {
        let mut cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Event, "event".to_string());

        cluster.add_memory("m1".to_string());
        cluster.remove_memory("nonexistent");

        assert_eq!(cluster.memory_ids.len(), 1);
    }

    #[test]
    fn test_cluster_calculate_importance_empty() {
        let cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Event, "event".to_string());

        let memories: Vec<&Memory> = vec![];
        let importance = cluster.calculate_importance(&memories);

        assert_eq!(importance, 0.0);
    }

    #[test]
    fn test_cluster_calculate_importance() {
        let cluster =
            MemoryCluster::new("Test".to_string(), ClusterType::Event, "event".to_string());

        let m1 = Memory::working("Test 1".to_string());
        let m2 = Memory::semantic("Test 2".to_string(), "concept".to_string());
        let memories: Vec<&Memory> = vec![&m1, &m2];

        let importance = cluster.calculate_importance(&memories);

        assert!(importance > 0.0);
        assert!(importance <= 1.0);
    }

    // ========================================================================
    // SERIALIZATION TESTS
    // ========================================================================

    #[test]
    fn test_memory_type_serialization() {
        let types = vec![
            MemoryType::Sensory,
            MemoryType::Working,
            MemoryType::Episodic,
            MemoryType::Semantic,
            MemoryType::Procedural,
            MemoryType::Emotional,
            MemoryType::Social,
        ];

        for mem_type in types {
            let json = serde_json::to_string(&mem_type).unwrap();
            let deserialized: MemoryType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, mem_type);
        }
    }

    #[test]
    fn test_memory_source_serialization() {
        let sources = vec![
            MemorySource::DirectExperience,
            MemorySource::Conversation,
            MemorySource::Learning,
            MemorySource::Inference,
            MemorySource::SharedMemory,
            MemorySource::SystemGenerated,
        ];

        for source in sources {
            let json = serde_json::to_string(&source).unwrap();
            let _deserialized: MemorySource = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_sharing_type_serialization() {
        let types = vec![
            SharingType::Full,
            SharingType::Summary,
            SharingType::Metadata,
            SharingType::Restricted,
        ];

        for sharing_type in types {
            let json = serde_json::to_string(&sharing_type).unwrap();
            let deserialized: SharingType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, sharing_type);
        }
    }

    #[test]
    fn test_privacy_level_serialization() {
        let levels = vec![
            PrivacyLevel::Public,
            PrivacyLevel::Group,
            PrivacyLevel::Personal,
            PrivacyLevel::Secret,
        ];

        for level in levels {
            let json = serde_json::to_string(&level).unwrap();
            let deserialized: PrivacyLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, level);
        }
    }

    #[test]
    fn test_cluster_type_serialization() {
        let types = vec![
            ClusterType::Event,
            ClusterType::Person,
            ClusterType::Location,
            ClusterType::Concept,
            ClusterType::Skill,
            ClusterType::Relationship,
        ];

        for cluster_type in types {
            let json = serde_json::to_string(&cluster_type).unwrap();
            let _deserialized: ClusterType = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_memory_full_serialization() {
        let memory = Memory::episodic(
            "Visited museum".to_string(),
            vec!["friend".to_string()],
            Some("museum".to_string()),
        );

        let json = serde_json::to_string(&memory).unwrap();
        let deserialized: Memory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.content.text, memory.content.text);
        assert_eq!(deserialized.memory_type, memory.memory_type);
    }

    // ========================================================================
    // EDGE CASE TESTS
    // ========================================================================

    #[test]
    fn test_emotional_context_structure() {
        let emotional = EmotionalContext {
            primary_emotion: "joy".to_string(),
            intensity: 0.8,
            valence: 0.9,
            arousal: 0.7,
        };

        let json = serde_json::to_string(&emotional).unwrap();
        let deserialized: EmotionalContext = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.primary_emotion, "joy");
        assert_eq!(deserialized.intensity, 0.8);
    }

    #[test]
    fn test_spatial_temporal_context_structure() {
        let context = SpatialTemporalContext {
            location: Some("park".to_string()),
            time_period: Some("afternoon".to_string()),
            duration: Some(3600000),
            participants: vec!["Alice".to_string()],
            related_events: vec!["picnic".to_string()],
        };

        let json = serde_json::to_string(&context).unwrap();
        let deserialized: SpatialTemporalContext = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.location, Some("park".to_string()));
        assert_eq!(deserialized.duration, Some(3600000));
    }

    #[test]
    fn test_consolidation_state() {
        let state = ConsolidationState {
            consolidation_level: 0.5,
            passes: 3,
            last_consolidation: Utc::now(),
            needs_consolidation: true,
            priority: 0.7,
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ConsolidationState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.consolidation_level, 0.5);
        assert_eq!(deserialized.passes, 3);
        assert!(deserialized.needs_consolidation);
    }

    #[test]
    fn test_forgetting_curve() {
        let curve = ForgettingCurve {
            initial_strength: 1.0,
            decay_rate: 0.1,
            half_life: 7.0,
            retention_threshold: 0.3,
            immune: false,
        };

        let json = serde_json::to_string(&curve).unwrap();
        let deserialized: ForgettingCurve = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.initial_strength, 1.0);
        assert!(!deserialized.immune);
    }

    #[test]
    fn test_time_window() {
        let window = TimeWindow {
            start: Utc::now() - chrono::Duration::days(7),
            end: Utc::now(),
        };

        let json = serde_json::to_string(&window).unwrap();
        let deserialized: TimeWindow = serde_json::from_str(&json).unwrap();

        assert!(deserialized.start < deserialized.end);
    }

    #[test]
    fn test_sharing_metadata() {
        let metadata = SharingMetadata {
            shareable: true,
            authorized_entities: vec!["agent1".to_string(), "agent2".to_string()],
            sharing_type: SharingType::Summary,
            privacy_level: PrivacyLevel::Group,
            sharing_conditions: vec!["must_be_online".to_string()],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SharingMetadata = serde_json::from_str(&json).unwrap();

        assert!(deserialized.shareable);
        assert_eq!(deserialized.authorized_entities.len(), 2);
        assert_eq!(deserialized.sharing_type, SharingType::Summary);
    }
}
