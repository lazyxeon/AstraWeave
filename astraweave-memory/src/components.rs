//! ECS components for memory system integration
//!
//! This module provides Bevy ECS components for integrating the memory system
//! with the game engine's entity-component architecture.

use crate::memory_manager::MemoryManager;
use crate::memory_types::*;
use bevy_ecs::component::Component;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Component that gives an entity access to the memory system
#[derive(Component, Debug, Clone)]
pub struct MemoryComponent {
    /// Entity's personal memory manager
    pub memory_manager: Arc<Mutex<MemoryManager>>,
    /// Entity's unique identifier for memory purposes
    pub entity_id: String,
    /// Memory configuration for this entity
    pub config: MemoryEntityConfig,
}

/// Configuration for an entity's memory capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntityConfig {
    /// Maximum number of memories to retain
    pub max_memories: usize,
    /// Memory types this entity can form
    pub allowed_memory_types: Vec<MemoryType>,
    /// Whether this entity can share memories
    pub can_share_memories: bool,
    /// Default importance threshold for memory formation
    pub memory_formation_threshold: f32,
    /// How often to run memory consolidation (in seconds)
    pub consolidation_interval: f32,
    /// How often to run forgetting processes (in seconds)
    pub forgetting_interval: f32,
}

impl Default for MemoryEntityConfig {
    fn default() -> Self {
        Self {
            max_memories: 1000,
            allowed_memory_types: vec![
                MemoryType::Sensory,
                MemoryType::Working,
                MemoryType::Episodic,
                MemoryType::Emotional,
                MemoryType::Social,
            ],
            can_share_memories: false,
            memory_formation_threshold: 0.3,
            consolidation_interval: 60.0, // 1 minute
            forgetting_interval: 300.0,   // 5 minutes
        }
    }
}

/// Component for entities that can form episodic memories
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryComponent {
    /// Current episodic context
    pub current_context: SpatialTemporalContext,
    /// Recent experiences that might form episodic memories
    pub pending_experiences: Vec<ExperienceBuffer>,
    /// Minimum importance for episodic memory formation
    pub episodic_threshold: f32,
}

/// Buffer for experiences that might become episodic memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceBuffer {
    /// Description of the experience
    pub description: String,
    /// When the experience occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Participants in the experience
    pub participants: Vec<String>,
    /// Location of the experience
    pub location: Option<String>,
    /// Emotional intensity of the experience
    pub emotional_intensity: f32,
    /// Importance score
    pub importance: f32,
}

impl Default for EpisodicMemoryComponent {
    fn default() -> Self {
        Self {
            current_context: SpatialTemporalContext {
                location: None,
                time_period: Some("present".to_string()),
                duration: None,
                participants: Vec::new(),
                related_events: Vec::new(),
            },
            pending_experiences: Vec::new(),
            episodic_threshold: 0.4,
        }
    }
}

/// Component for entities that can form working memory
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryComponent {
    /// Current tasks in working memory
    pub active_tasks: Vec<WorkingMemoryTask>,
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// How long to retain working memory (in seconds)
    pub retention_duration: f32,
}

/// A task or piece of information in working memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryTask {
    /// Task identifier
    pub id: String,
    /// Task description
    pub description: String,
    /// When the task was added to working memory
    pub added_at: chrono::DateTime<chrono::Utc>,
    /// Priority level (0.0 to 1.0)
    pub priority: f32,
    /// Associated data
    pub data: serde_json::Value,
    /// Whether this task is currently active
    pub active: bool,
}

impl Default for WorkingMemoryComponent {
    fn default() -> Self {
        Self {
            active_tasks: Vec::new(),
            max_concurrent_tasks: 7,   // Miller's magic number
            retention_duration: 300.0, // 5 minutes
        }
    }
}

/// Component for entities that can form social memories
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SocialMemoryComponent {
    /// Known entities and relationships
    pub known_entities: std::collections::HashMap<String, SocialRelationship>,
    /// Social context information
    pub social_context: SocialContext,
    /// Recent social interactions
    pub recent_interactions: Vec<SocialInteraction>,
}

/// Information about a social relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialRelationship {
    /// Entity identifier
    pub entity_id: String,
    /// Name or identifier for display
    pub name: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength of relationship (0.0 to 1.0)
    pub strength: f32,
    /// Trust level (0.0 to 1.0)
    pub trust: f32,
    /// When this relationship was first established
    pub established_at: chrono::DateTime<chrono::Utc>,
    /// Last interaction time
    pub last_interaction: chrono::DateTime<chrono::Utc>,
    /// Number of interactions
    pub interaction_count: u32,
}

/// Types of social relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RelationshipType {
    Friend,
    Colleague,
    Family,
    Acquaintance,
    Enemy,
    Neutral,
    Authority,
    Subordinate,
}

/// Current social context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialContext {
    /// Current group or social setting
    pub current_group: Option<String>,
    /// Social norms in current context
    pub active_norms: Vec<String>,
    /// Social roles currently active
    pub active_roles: Vec<String>,
}

/// Record of a social interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialInteraction {
    /// Who was involved in the interaction
    pub participants: Vec<String>,
    /// Type of interaction
    pub interaction_type: InteractionType,
    /// When it occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Brief description
    pub description: String,
    /// Emotional valence of the interaction
    pub valence: f32,
    /// Importance of the interaction
    pub importance: f32,
}

/// Types of social interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InteractionType {
    Conversation,
    Collaboration,
    Conflict,
    Help,
    Trade,
    Greeting,
    Farewell,
    Other(String),
}

impl Default for SocialMemoryComponent {
    fn default() -> Self {
        Self {
            known_entities: std::collections::HashMap::new(),
            social_context: SocialContext {
                current_group: None,
                active_norms: Vec::new(),
                active_roles: Vec::new(),
            },
            recent_interactions: Vec::new(),
        }
    }
}

/// Component for memory-based learning and adaptation
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct LearningMemoryComponent {
    /// Skills or knowledge areas being learned
    pub learning_domains: Vec<LearningDomain>,
    /// Performance metrics over time
    pub performance_history: Vec<PerformanceRecord>,
    /// Learning rate parameters
    pub learning_rate: f32,
    /// Forgetting rate for learned skills
    pub skill_decay_rate: f32,
}

/// A domain of learning or skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningDomain {
    /// Domain name/identifier
    pub name: String,
    /// Current proficiency level (0.0 to 1.0)
    pub proficiency: f32,
    /// Practice sessions in this domain
    pub practice_sessions: Vec<PracticeSession>,
    /// When this domain was first encountered
    pub first_encountered: chrono::DateTime<chrono::Utc>,
    /// Last practice time
    pub last_practice: chrono::DateTime<chrono::Utc>,
}

/// Record of a practice session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeSession {
    /// When the session occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Duration in seconds
    pub duration: f32,
    /// Performance score in this session
    pub performance_score: f32,
    /// Difficulty level practiced
    pub difficulty: f32,
}

/// Record of performance in a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecord {
    /// Task or skill identifier
    pub task_id: String,
    /// Performance score
    pub score: f32,
    /// When this performance was recorded
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Context or conditions
    pub context: String,
}

impl Default for LearningMemoryComponent {
    fn default() -> Self {
        Self {
            learning_domains: Vec::new(),
            performance_history: Vec::new(),
            learning_rate: 0.1,
            skill_decay_rate: 0.01,
        }
    }
}

/// Component that tracks memory-related events for debugging
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDebugComponent {
    /// Recent memory operations
    pub recent_operations: Vec<MemoryOperation>,
    /// Memory statistics
    pub statistics: MemoryStatistics,
    /// Debug flags
    pub debug_flags: MemoryDebugFlags,
}

/// Record of a memory operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperation {
    /// Type of operation
    pub operation_type: MemoryOperationType,
    /// Memory ID involved
    pub memory_id: String,
    /// When the operation occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Success status
    pub success: bool,
    /// Additional details
    pub details: String,
}

/// Types of memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MemoryOperationType {
    Create,
    Retrieve,
    Update,
    Delete,
    Consolidate,
    Forget,
    Share,
    Associate,
}

/// Memory statistics for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    /// Total number of memories
    pub total_memories: usize,
    /// Memories by type
    pub memories_by_type: std::collections::HashMap<String, usize>,
    /// Average memory strength
    pub average_strength: f32,
    /// Memory formation rate (per hour)
    pub formation_rate: f32,
    /// Forgetting rate (per hour)
    pub forgetting_rate: f32,
}

/// Debug flags for memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDebugFlags {
    /// Log all memory operations
    pub log_operations: bool,
    /// Visualize memory networks
    pub visualize_networks: bool,
    /// Track performance metrics
    pub track_performance: bool,
    /// Enable memory introspection
    pub enable_introspection: bool,
}

impl Default for MemoryDebugComponent {
    fn default() -> Self {
        Self {
            recent_operations: Vec::new(),
            statistics: MemoryStatistics {
                total_memories: 0,
                memories_by_type: std::collections::HashMap::new(),
                average_strength: 0.0,
                formation_rate: 0.0,
                forgetting_rate: 0.0,
            },
            debug_flags: MemoryDebugFlags {
                log_operations: false,
                visualize_networks: false,
                track_performance: false,
                enable_introspection: false,
            },
        }
    }
}

impl MemoryComponent {
    /// Create a new memory component for an entity
    pub fn new(entity_id: String, config: MemoryEntityConfig) -> Self {
        let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));

        Self {
            memory_manager,
            entity_id,
            config,
        }
    }

    /// Get a reference to the memory manager
    pub fn get_memory_manager(&self) -> Arc<Mutex<MemoryManager>> {
        self.memory_manager.clone()
    }
}

impl WorkingMemoryComponent {
    /// Add a new task to working memory
    pub fn add_task(
        &mut self,
        description: String,
        priority: f32,
        data: serde_json::Value,
    ) -> String {
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = WorkingMemoryTask {
            id: task_id.clone(),
            description,
            added_at: chrono::Utc::now(),
            priority: priority.clamp(0.0, 1.0),
            data,
            active: true,
        };

        // Remove lowest priority task if at capacity
        if self.active_tasks.len() >= self.max_concurrent_tasks {
            self.active_tasks.sort_by(|a, b| {
                a.priority
                    .partial_cmp(&b.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.active_tasks.remove(0);
        }

        self.active_tasks.push(task);
        task_id
    }

    /// Remove a task from working memory
    pub fn remove_task(&mut self, task_id: &str) {
        self.active_tasks.retain(|task| task.id != task_id);
    }

    /// Clean up old tasks based on retention duration
    pub fn cleanup_old_tasks(&mut self) {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::seconds(self.retention_duration as i64);

        self.active_tasks.retain(|task| task.added_at > cutoff);
    }
}

impl SocialMemoryComponent {
    /// Add or update a social relationship
    pub fn update_relationship(
        &mut self,
        entity_id: String,
        name: String,
        relationship_type: RelationshipType,
    ) {
        let relationship =
            self.known_entities
                .entry(entity_id.clone())
                .or_insert(SocialRelationship {
                    entity_id: entity_id.clone(),
                    name,
                    relationship_type,
                    strength: 0.5,
                    trust: 0.5,
                    established_at: chrono::Utc::now(),
                    last_interaction: chrono::Utc::now(),
                    interaction_count: 0,
                });

        relationship.last_interaction = chrono::Utc::now();
        relationship.interaction_count += 1;
    }

    /// Record a social interaction
    pub fn record_interaction(&mut self, interaction: SocialInteraction) {
        // Update relationships based on interaction
        for participant in &interaction.participants {
            if let Some(relationship) = self.known_entities.get_mut(participant) {
                relationship.last_interaction = interaction.timestamp;

                // Adjust relationship strength based on interaction valence
                let strength_adjustment = interaction.valence * 0.1;
                relationship.strength =
                    (relationship.strength + strength_adjustment).clamp(0.0, 1.0);
            }
        }

        self.recent_interactions.push(interaction);

        // Keep only recent interactions (last 100)
        if self.recent_interactions.len() > 100 {
            self.recent_interactions.remove(0);
        }
    }
}

impl LearningMemoryComponent {
    /// Record a practice session in a learning domain
    pub fn record_practice(
        &mut self,
        domain_name: String,
        duration: f32,
        performance_score: f32,
        difficulty: f32,
    ) {
        let session = PracticeSession {
            timestamp: chrono::Utc::now(),
            duration,
            performance_score,
            difficulty,
        };

        // Find or create the learning domain
        let domain_exists = self.learning_domains.iter().any(|d| d.name == domain_name);
        if !domain_exists {
            self.learning_domains.push(LearningDomain {
                name: domain_name.clone(),
                proficiency: 0.0,
                practice_sessions: Vec::new(),
                first_encountered: chrono::Utc::now(),
                last_practice: chrono::Utc::now(),
            });
        }

        let domain = self
            .learning_domains
            .iter_mut()
            .find(|d| d.name == domain_name)
            .expect("domain was just created above");

        domain.practice_sessions.push(session);
        domain.last_practice = chrono::Utc::now();

        // Update proficiency based on performance
        let learning_adjustment = performance_score * self.learning_rate;
        domain.proficiency = (domain.proficiency + learning_adjustment).clamp(0.0, 1.0);
    }
}
