use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use astraweave_context::{ContextConfig, ConversationHistory};
use astraweave_llm::LlmClient;
use astraweave_prompts::library::PromptLibrary;
use astraweave_prompts::template::PromptTemplate;
use astraweave_rag::RagPipeline;

/// Narrative coherence engine for maintaining story consistency across all game systems
#[allow(dead_code)] // Fields reserved for RAG-enhanced coherence pipeline
pub struct NarrativeCoherenceEngine {
    llm_client: Arc<dyn LlmClient>,
    rag_pipeline: Arc<RagPipeline>,
    conversation_history: Arc<RwLock<ConversationHistory>>,
    prompt_library: Arc<RwLock<PromptLibrary>>,
    config: CoherenceConfig,
    narrative_state: Arc<RwLock<NarrativeState>>,
    consistency_rules: Arc<RwLock<Vec<ConsistencyRule>>>,
    story_threads: Arc<RwLock<HashMap<String, StoryThread>>>,
    character_arcs: Arc<RwLock<HashMap<String, CharacterArc>>>,
    world_continuity: Arc<RwLock<WorldContinuity>>,
}

/// Configuration for narrative coherence
#[derive(Debug, Clone)]
pub struct CoherenceConfig {
    pub enable_real_time_validation: bool,
    pub coherence_threshold: f32,
    pub max_story_threads: usize,
    pub character_consistency_weight: f32,
    pub world_consistency_weight: f32,
    pub temporal_consistency_weight: f32,
    pub context_window_size: usize,
}

impl Default for CoherenceConfig {
    fn default() -> Self {
        Self {
            enable_real_time_validation: true,
            coherence_threshold: 0.8,
            max_story_threads: 10,
            character_consistency_weight: 0.4,
            world_consistency_weight: 0.3,
            temporal_consistency_weight: 0.3,
            context_window_size: 4096,
        }
    }
}

/// Current narrative state of the game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeState {
    pub main_storyline: Option<String>,
    pub active_plot_points: Vec<PlotPoint>,
    pub resolved_conflicts: Vec<String>,
    pub character_states: HashMap<String, CharacterState>,
    pub world_events: Vec<NarrativeEvent>,
    pub timeline: Timeline,
    pub narrative_themes: Vec<String>,
    pub tone: NarrativeTone,
    pub pacing: PacingState,
}

/// Individual plot point in the narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotPoint {
    pub id: String,
    pub description: String,
    pub plot_type: PlotType,
    pub importance: f32,
    pub status: PlotStatus,
    pub related_characters: Vec<String>,
    pub prerequisites: Vec<String>,
    pub consequences: Vec<String>,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub introduced_at: time::OffsetDateTime,
    #[serde(default, with = "crate::utc_stamp::serde_rfc3339::option")]
    pub deadline: Option<time::OffsetDateTime>,
}

/// Types of plot points
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum PlotType {
    MainQuest,
    SideQuest,
    CharacterDevelopment,
    WorldBuilding,
    Conflict,
    Resolution,
    Revelation,
    Foreshadowing,
}

/// Status of plot points
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum PlotStatus {
    Dormant,    // Not yet active
    Active,     // Currently relevant
    Developing, // Building toward resolution
    Resolved,   // Completed
    Abandoned,  // No longer relevant
}

/// State of a character in the narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterState {
    pub character_id: String,
    pub current_motivation: String,
    pub emotional_state: EmotionalState,
    pub relationships: HashMap<String, Relationship>,
    pub character_arc_progress: f32, // 0.0 to 1.0
    pub secrets_known: Vec<String>,
    pub goals: Vec<String>,
    pub fears: Vec<String>,
    pub last_significant_event: Option<String>,
}

/// Emotional state of a character
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub primary_emotion: String,
    pub intensity: f32, // 0.0 to 1.0
    pub stability: f32, // How quickly emotions change
    pub recent_changes: Vec<EmotionalChange>,
}

/// Change in character emotional state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalChange {
    pub from_emotion: String,
    pub to_emotion: String,
    pub cause: String,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
}

/// Relationship between characters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub other_character: String,
    pub relationship_type: RelationshipType,
    pub strength: f32, // -1.0 (enemy) to 1.0 (ally)
    pub trust_level: f32,
    pub shared_history: Vec<String>,
    pub current_status: String,
}

/// Types of relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RelationshipType {
    Family,
    Friend,
    Romantic,
    Professional,
    Rival,
    Enemy,
    Mentor,
    Student,
    Unknown,
}

/// Narrative event for coherence tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    pub id: String,
    pub event_type: String,
    pub description: String,
    pub participants: Vec<String>,
    pub consequences: Vec<String>,
    pub narrative_impact: f32,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
    pub location: Option<String>,
    pub related_plot_points: Vec<String>,
}

/// Game timeline for temporal consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub current_time: time::OffsetDateTime,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub game_start_time: time::OffsetDateTime,
    pub significant_dates: Vec<TimelineEvent>,
    pub temporal_anchors: Vec<TemporalAnchor>,
}

/// Significant event in the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub date: time::OffsetDateTime,
    pub title: String,
    pub description: String,
    pub importance: f32,
    pub participants: Vec<String>,
}

/// Fixed point in time for consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnchor {
    pub id: String,
    pub description: String,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub fixed_date: time::OffsetDateTime,
    pub cannot_change: bool,
    pub related_events: Vec<String>,
}

/// Overall narrative tone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeTone {
    pub primary_tone: String,
    pub secondary_tones: Vec<String>,
    pub mood_spectrum: f32, // -1.0 (dark) to 1.0 (light)
    pub tension_level: f32,
    pub comedy_level: f32,
}

/// Pacing state of the narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingState {
    pub current_pace: PaceType,
    pub recent_events_intensity: f32,
    pub time_since_major_event: u64, // milliseconds
    pub player_engagement_level: f32,
}

/// Types of narrative pacing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PaceType {
    Slow,
    Steady,
    Building,
    Climactic,
    Resolving,
}

/// Rule for maintaining narrative consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyRule {
    pub id: String,
    pub name: String,
    pub rule_type: RuleType,
    pub condition: String,
    pub validation_logic: String,
    pub priority: f32,
    pub active: bool,
    pub violations: Vec<RuleViolation>,
}

/// Types of consistency rules
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RuleType {
    Character,
    World,
    Temporal,
    Plot,
    Dialogue,
    Causality,
}

/// Violation of a consistency rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleViolation {
    pub violation_id: String,
    pub description: String,
    pub severity: ViolationSeverity,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub detected_at: time::OffsetDateTime,
    pub resolved: bool,
    pub resolution_notes: Option<String>,
}

/// Severity of rule violations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ViolationSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Story thread connecting related narrative elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryThread {
    pub id: String,
    pub title: String,
    pub theme: String,
    pub related_plot_points: Vec<String>,
    pub characters_involved: Vec<String>,
    pub current_status: ThreadStatus,
    pub importance: f32,
    #[serde(default, with = "crate::utc_stamp::serde_rfc3339::option")]
    pub estimated_resolution_time: Option<time::OffsetDateTime>,
    pub branching_possibilities: Vec<String>,
}

/// Status of story threads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ThreadStatus {
    Introduced,
    Developing,
    Climax,
    Resolving,
    Resolved,
    Abandoned,
}

/// Character arc tracking character development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterArc {
    pub character_id: String,
    pub arc_type: String,
    pub starting_state: String,
    pub desired_end_state: String,
    pub current_progress: f32,
    pub milestones: Vec<ArcMilestone>,
    pub obstacles: Vec<String>,
    pub growth_catalysts: Vec<String>,
}

/// Milestone in character development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcMilestone {
    pub description: String,
    pub achieved: bool,
    #[serde(default, with = "crate::utc_stamp::serde_rfc3339::option")]
    pub achievement_date: Option<time::OffsetDateTime>,
    pub impact_on_character: String,
}

/// World continuity tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldContinuity {
    pub established_facts: HashMap<String, EstablishedFact>,
    pub location_states: HashMap<String, LocationState>,
    pub faction_dynamics: HashMap<String, FactionState>,
    pub economic_continuity: EconomicContinuity,
    pub cultural_elements: HashMap<String, CulturalElement>,
}

/// Established fact about the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstablishedFact {
    pub id: String,
    pub description: String,
    pub category: String,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub established_at: time::OffsetDateTime,
    pub immutable: bool,
    pub supporting_evidence: Vec<String>,
    pub contradictions: Vec<String>,
}

/// State of a location for continuity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationState {
    pub location_id: String,
    pub current_description: String,
    pub notable_features: Vec<String>,
    pub recent_changes: Vec<LocationChange>,
    pub inhabitants: Vec<String>,
    pub accessibility: String,
}

/// Change to a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationChange {
    pub change_type: String,
    pub description: String,
    pub cause: String,
    #[serde(with = "crate::utc_stamp::serde_rfc3339")]
    pub timestamp: time::OffsetDateTime,
    pub reversible: bool,
}

/// State of a faction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionState {
    pub faction_id: String,
    pub current_goals: Vec<String>,
    pub resources: HashMap<String, f32>,
    pub relationships: HashMap<String, f32>,
    pub recent_actions: Vec<String>,
    pub leadership_changes: Vec<String>,
}

/// Economic continuity tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicContinuity {
    pub market_trends: HashMap<String, f32>,
    pub trade_routes: Vec<TradeRoute>,
    pub economic_events: Vec<String>,
    pub currency_stability: f32,
}

/// Trade route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRoute {
    pub id: String,
    pub from_location: String,
    pub to_location: String,
    pub goods_traded: Vec<String>,
    pub current_status: String,
    pub safety_level: f32,
}

/// Cultural element for world consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalElement {
    pub id: String,
    pub element_type: String, // tradition, belief, custom, etc.
    pub description: String,
    pub associated_groups: Vec<String>,
    pub importance: f32,
    pub evolution_over_time: Vec<String>,
}

impl NarrativeCoherenceEngine {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        rag_pipeline: Arc<RagPipeline>,
        config: CoherenceConfig,
    ) -> Result<Self> {
        let context_config = ContextConfig {
            max_tokens: config.context_window_size,
            ..Default::default()
        };
        let conversation_history = Arc::new(RwLock::new(ConversationHistory::new(context_config)));

        let mut prompt_library = PromptLibrary::new();

        // Load narrative coherence prompts
        prompt_library.add_template("coherence_analysis", PromptTemplate::new("coherence_analysis".to_string(),
            r#"
You are a narrative coherence analyzer. Examine the proposed narrative element for consistency with established story elements.

Current Narrative State:
- Main Storyline: {{main_storyline}}
- Active Plot Points: {{active_plot_points}}
- Character States: {{character_states}}
- Recent Events: {{recent_events}}
- Established Facts: {{established_facts}}

Proposed Element: {{proposed_element}}

Analyze coherence in JSON format:
{
    "coherence_score": 0.85,
    "character_consistency": 0.9,
    "world_consistency": 0.8,
    "temporal_consistency": 0.85,
    "issues": [
        {
            "type": "character",
            "severity": "minor",
            "description": "Character behavior inconsistent with established personality",
            "suggestion": "Adjust dialogue to reflect character's cautious nature"
        }
    ],
    "strengths": [
        "Well integrated with ongoing plot threads",
        "Maintains established world rules"
    ],
    "narrative_impact": 0.7,
    "recommendations": [
        "Add reference to previous character interaction",
        "Include consequence for established faction relationship"
    ]
}

Focus on:
1. Character behavior consistency
2. World rule adherence
3. Timeline coherence
4. Plot thread continuity
5. Established fact preservation
            "#.trim().to_string()
        ));

        prompt_library.add_template("story_thread_weaving", PromptTemplate::new("story_thread_weaving".to_string(),
            r#"
You are connecting narrative elements into coherent story threads. Analyze how different plot points can be woven together.

Active Plot Points: {{plot_points}}
Character Arcs: {{character_arcs}}
Current Story Threads: {{story_threads}}

Create story thread connections in JSON format:
{
    "thread_connections": [
        {
            "thread_id": "power_struggle",
            "connected_plots": ["noble_conspiracy", "merchant_rebellion"],
            "connection_strength": 0.8,
            "narrative_tension": 0.9,
            "resolution_path": "Political confrontation leading to compromise"
        }
    ],
    "new_threads": [
        {
            "title": "The Hidden Truth",
            "theme": "revelation",
            "plot_points": ["ancient_prophecy", "mysterious_artifact"],
            "estimated_duration": "3-5 sessions",
            "climax_potential": 0.85
        }
    ],
    "thread_conflicts": [
        {
            "thread1": "romance_subplot",
            "thread2": "duty_conflict",
            "conflict_type": "character_motivation",
            "resolution_needed": true
        }
    ]
}

Create meaningful connections that enhance the overall narrative.
            "#.trim().to_string()
        ));

        prompt_library.add_template("character_arc_guidance", PromptTemplate::new("character_arc_guidance".to_string(),
            r#"
You are guiding character development for narrative coherence. Analyze character progression and suggest developments.

Character: {{character_id}}
Current State: {{current_state}}
Character Arc: {{character_arc}}
Recent Interactions: {{recent_interactions}}
Relationship Changes: {{relationship_changes}}

Provide character development guidance in JSON format:
{
    "arc_progress_assessment": {
        "current_milestone": "accepting_responsibility",
        "progress_percentage": 0.65,
        "next_development_needed": "confronting_past_mistakes",
        "timeline_appropriateness": 0.9
    },
    "suggested_developments": [
        {
            "development_type": "emotional_growth",
            "description": "Character shows vulnerability to trusted ally",
            "narrative_timing": "during_quiet_moment",
            "impact_on_relationships": ["strengthens_bond_with_mentor"]
        }
    ],
    "consistency_concerns": [
        {
            "concern": "Recent aggressive behavior conflicts with established compassionate nature",
            "severity": "moderate",
            "suggestion": "Provide internal conflict justification or stress-based explanation"
        }
    ],
    "relationship_dynamics": {
        "strongest_bond": "mentor_figure",
        "most_tension": "authority_figure",
        "development_opportunities": ["reconciliation_with_sibling"]
    }
}

Ensure character development feels natural and earned.
            "#.trim().to_string()
        ));

        let narrative_state = NarrativeState {
            main_storyline: None,
            active_plot_points: Vec::new(),
            resolved_conflicts: Vec::new(),
            character_states: HashMap::new(),
            world_events: Vec::new(),
            timeline: Timeline {
                current_time: time::OffsetDateTime::now_utc(),
                game_start_time: time::OffsetDateTime::now_utc(),
                significant_dates: Vec::new(),
                temporal_anchors: Vec::new(),
            },
            narrative_themes: Vec::new(),
            tone: NarrativeTone {
                primary_tone: "adventure".to_string(),
                secondary_tones: Vec::new(),
                mood_spectrum: 0.0,
                tension_level: 0.5,
                comedy_level: 0.3,
            },
            pacing: PacingState {
                current_pace: PaceType::Steady,
                recent_events_intensity: 0.5,
                time_since_major_event: 0,
                player_engagement_level: 0.7,
            },
        };

        Ok(Self {
            llm_client,
            rag_pipeline,
            conversation_history,
            prompt_library: Arc::new(RwLock::new(prompt_library)),
            config,
            narrative_state: Arc::new(RwLock::new(narrative_state)),
            consistency_rules: Arc::new(RwLock::new(Vec::new())),
            story_threads: Arc::new(RwLock::new(HashMap::new())),
            character_arcs: Arc::new(RwLock::new(HashMap::new())),
            world_continuity: Arc::new(RwLock::new(WorldContinuity {
                established_facts: HashMap::new(),
                location_states: HashMap::new(),
                faction_dynamics: HashMap::new(),
                economic_continuity: EconomicContinuity {
                    market_trends: HashMap::new(),
                    trade_routes: Vec::new(),
                    economic_events: Vec::new(),
                    currency_stability: 1.0,
                },
                cultural_elements: HashMap::new(),
            })),
        })
    }

    /// Validate narrative coherence of a proposed element
    pub async fn validate_coherence(
        &self,
        proposed_element: &serde_json::Value,
    ) -> Result<CoherenceAnalysis> {
        debug!("Validating narrative coherence");

        if !self.config.enable_real_time_validation {
            return Ok(CoherenceAnalysis::default());
        }

        // Build context for analysis
        let context = self.build_coherence_context().await?;
        let mut analysis_context = context;
        analysis_context.insert("proposed_element".to_string(), proposed_element.clone());

        // Analyze coherence using LLM
        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("coherence_analysis")?;

        // Serialize context values into strings for template rendering
        let mut serialized_context: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in &analysis_context {
            serialized_context.insert(
                k.clone(),
                serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()),
            );
        }

        let prompt = template.render_map(&serialized_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Coherence analysis failed: {}", e))?;

        let analysis: CoherenceAnalysis = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse coherence analysis: {}", e))?;

        // Check against consistency rules
        let rule_violations = self.check_consistency_rules(proposed_element).await?;

        // Combine LLM analysis with rule-based checks
        let final_analysis = self
            .combine_coherence_results(analysis, rule_violations)
            .await;

        if final_analysis.coherence_score < self.config.coherence_threshold {
            warn!(
                "Narrative element failed coherence check: {:.2}",
                final_analysis.coherence_score
            );
        } else {
            debug!(
                "Narrative element passed coherence check: {:.2}",
                final_analysis.coherence_score
            );
        }

        Ok(final_analysis)
    }

    /// Weave story threads together for better narrative cohesion
    pub async fn weave_story_threads(&self) -> Result<ThreadWeavingResult> {
        debug!("Weaving story threads for narrative cohesion");

        let context = self.build_thread_context().await?;
        // Convert Value context to String context for render_map
        let string_context: HashMap<String, String> = context
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("story_thread_weaving")?;
        let prompt = template.render_map(&string_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Story thread weaving failed: {}", e))?;

        let result: ThreadWeavingResult = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse thread weaving result: {}", e))?;

        // Apply thread connections
        self.apply_thread_connections(&result).await?;

        info!(
            "Wove {} story threads with {} new connections",
            result.thread_connections.len(),
            result.new_threads.len()
        );

        Ok(result)
    }

    /// Guide character development for narrative coherence
    pub async fn guide_character_development(
        &self,
        character_id: &str,
    ) -> Result<CharacterGuidance> {
        debug!(
            "Providing character development guidance for {}",
            character_id
        );

        let context = self.build_character_context(character_id).await?;
        // Convert Value context to String context for render_map
        let string_context: HashMap<String, String> = context
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("character_arc_guidance")?;
        let prompt = template.render_map(&string_context)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Character development guidance failed: {}", e))?;

        let guidance: CharacterGuidance = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse character guidance: {}", e))?;

        // Update character arc if recommendations are valid
        if guidance.arc_progress_assessment.timeline_appropriateness > 0.7 {
            self.update_character_arc(character_id, &guidance).await?;
        }

        info!(
            "Provided character development guidance for {}",
            character_id
        );
        Ok(guidance)
    }

    /// Add a new plot point to the narrative
    pub async fn add_plot_point(&self, plot_point: PlotPoint) -> Result<()> {
        debug!("Adding plot point: {}", plot_point.description);

        // Validate coherence
        let validation = self
            .validate_coherence(&serde_json::to_value(&plot_point)?)
            .await?;

        if validation.coherence_score < self.config.coherence_threshold {
            return Err(anyhow!(
                "Plot point fails coherence requirements: {:.2}",
                validation.coherence_score
            ));
        }

        // Add to narrative state
        let mut narrative_state = self.narrative_state.write().await;
        narrative_state.active_plot_points.push(plot_point.clone());

        // Update related story threads
        self.update_story_threads_for_plot(&plot_point).await?;

        // Note: RAG memory storage would require &mut self, skip for now
        // as narrative coherence engine uses Arc<RagPipeline>
        // The plot point is already stored in narrative_state for reference
        debug!("Plot point stored in narrative state (RAG integration pending mut access)");

        info!("Added plot point: {}", plot_point.description);
        Ok(())
    }

    /// Update character state
    pub async fn update_character_state(
        &self,
        character_id: String,
        state: CharacterState,
    ) -> Result<()> {
        debug!("Updating character state for {}", character_id);

        // Validate character consistency
        let current_state = {
            let narrative_state = self.narrative_state.read().await;
            narrative_state.character_states.get(&character_id).cloned()
        };

        if let Some(current) = current_state {
            let consistency_score = self.calculate_character_consistency(&current, &state).await;
            if consistency_score < 0.7 {
                warn!(
                    "Character state change may be inconsistent: {:.2}",
                    consistency_score
                );
            }
        }

        // Update state
        let mut narrative_state = self.narrative_state.write().await;
        narrative_state
            .character_states
            .insert(character_id.clone(), state.clone());

        // Update character arc progress
        if let Some(arc) = self.character_arcs.read().await.get(&character_id) {
            let progress_change = state.character_arc_progress - arc.current_progress;
            if progress_change.abs() > 0.1 {
                debug!(
                    "Significant character arc progress for {}: {:.2}",
                    character_id, progress_change
                );
            }
        }

        info!("Updated character state for {}", character_id);
        Ok(())
    }

    /// Record a narrative event
    pub async fn record_narrative_event(&self, event: NarrativeEvent) -> Result<()> {
        debug!("Recording narrative event: {}", event.description);

        // Update narrative state
        let mut narrative_state = self.narrative_state.write().await;
        narrative_state.world_events.push(event.clone());

        // Update timeline
        narrative_state
            .timeline
            .significant_dates
            .push(TimelineEvent {
                date: event.timestamp,
                title: event.event_type.clone(),
                description: event.description.clone(),
                importance: event.narrative_impact,
                participants: event.participants.clone(),
            });

        // Update pacing based on event impact
        self.update_pacing_state(event.narrative_impact).await;

        // Check for narrative consequences
        self.process_event_consequences(&event).await?;

        info!("Recorded narrative event: {}", event.description);
        Ok(())
    }

    /// Get current narrative summary
    pub async fn get_narrative_summary(&self) -> NarrativeSummary {
        let narrative_state = self.narrative_state.read().await;
        let story_threads = self.story_threads.read().await;
        let character_arcs = self.character_arcs.read().await;

        NarrativeSummary {
            main_storyline: narrative_state.main_storyline.clone(),
            active_plot_count: narrative_state.active_plot_points.len(),
            character_count: narrative_state.character_states.len(),
            story_thread_count: story_threads.len(),
            character_arc_count: character_arcs.len(),
            narrative_tone: narrative_state.tone.clone(),
            pacing_state: narrative_state.pacing.clone(),
            coherence_health: self.calculate_overall_coherence().await,
        }
    }

    /// Build context for coherence analysis
    async fn build_coherence_context(&self) -> Result<HashMap<String, serde_json::Value>> {
        let narrative_state = self.narrative_state.read().await;
        let world_continuity = self.world_continuity.read().await;

        let mut context = HashMap::new();

        context.insert(
            "main_storyline".to_string(),
            serde_json::to_value(&narrative_state.main_storyline)?,
        );
        context.insert(
            "active_plot_points".to_string(),
            serde_json::to_value(&narrative_state.active_plot_points)?,
        );
        context.insert(
            "character_states".to_string(),
            serde_json::to_value(&narrative_state.character_states)?,
        );
        context.insert(
            "recent_events".to_string(),
            serde_json::to_value(
                narrative_state
                    .world_events
                    .iter()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>(),
            )?,
        );
        context.insert(
            "established_facts".to_string(),
            serde_json::to_value(&world_continuity.established_facts)?,
        );

        Ok(context)
    }

    /// Build context for story thread weaving
    async fn build_thread_context(&self) -> Result<HashMap<String, serde_json::Value>> {
        let narrative_state = self.narrative_state.read().await;
        let story_threads = self.story_threads.read().await;
        let character_arcs = self.character_arcs.read().await;

        let mut context = HashMap::new();

        context.insert(
            "plot_points".to_string(),
            serde_json::to_value(&narrative_state.active_plot_points)?,
        );
        context.insert(
            "character_arcs".to_string(),
            serde_json::to_value(&*character_arcs)?,
        );
        context.insert(
            "story_threads".to_string(),
            serde_json::to_value(&*story_threads)?,
        );

        Ok(context)
    }

    /// Build context for character development
    async fn build_character_context(
        &self,
        character_id: &str,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let narrative_state = self.narrative_state.read().await;
        let character_arcs = self.character_arcs.read().await;

        let mut context = HashMap::new();

        context.insert(
            "character_id".to_string(),
            serde_json::to_value(character_id)?,
        );

        if let Some(state) = narrative_state.character_states.get(character_id) {
            context.insert("current_state".to_string(), serde_json::to_value(state)?);
        }

        if let Some(arc) = character_arcs.get(character_id) {
            context.insert("character_arc".to_string(), serde_json::to_value(arc)?);
        }

        // Get recent interactions
        let recent_interactions: Vec<_> = narrative_state
            .world_events
            .iter()
            .filter(|event| event.participants.contains(&character_id.to_string()))
            .rev()
            .take(5)
            .collect();
        context.insert(
            "recent_interactions".to_string(),
            serde_json::to_value(recent_interactions)?,
        );

        // Get relationship changes
        if let Some(state) = narrative_state.character_states.get(character_id) {
            context.insert(
                "relationship_changes".to_string(),
                serde_json::to_value(&state.relationships)?,
            );
        }

        Ok(context)
    }

    /// Check consistency rules against proposed element
    async fn check_consistency_rules(
        &self,
        _element: &serde_json::Value,
    ) -> Result<Vec<RuleViolation>> {
        let _rules = self.consistency_rules.read().await;

        // For now, return empty violations
        // In practice, would evaluate rules against the element
        Ok(Vec::new())
    }

    /// Combine LLM analysis with rule-based checks
    async fn combine_coherence_results(
        &self,
        llm_analysis: CoherenceAnalysis,
        rule_violations: Vec<RuleViolation>,
    ) -> CoherenceAnalysis {
        let mut combined_analysis = llm_analysis;

        // Reduce coherence score for rule violations
        for violation in rule_violations {
            let penalty = match violation.severity {
                ViolationSeverity::Minor => 0.05,
                ViolationSeverity::Moderate => 0.1,
                ViolationSeverity::Major => 0.2,
                ViolationSeverity::Critical => 0.4,
            };
            combined_analysis.coherence_score -= penalty;
        }

        combined_analysis.coherence_score = combined_analysis.coherence_score.max(0.0);
        combined_analysis
    }

    /// Apply story thread connections
    async fn apply_thread_connections(&self, result: &ThreadWeavingResult) -> Result<()> {
        let mut story_threads = self.story_threads.write().await;

        // Add new threads
        for new_thread in &result.new_threads {
            let thread = StoryThread {
                id: Uuid::new_v4().to_string(),
                title: new_thread.title.clone(),
                theme: new_thread.theme.clone(),
                related_plot_points: new_thread.plot_points.clone(),
                characters_involved: Vec::new(),
                current_status: ThreadStatus::Introduced,
                importance: new_thread.climax_potential,
                estimated_resolution_time: None,
                branching_possibilities: Vec::new(),
            };
            story_threads.insert(thread.id.clone(), thread);
        }

        info!(
            "Applied {} new story thread connections",
            result.new_threads.len()
        );
        Ok(())
    }

    /// Update character arc based on guidance
    async fn update_character_arc(
        &self,
        character_id: &str,
        guidance: &CharacterGuidance,
    ) -> Result<()> {
        let mut character_arcs = self.character_arcs.write().await;

        if let Some(arc) = character_arcs.get_mut(character_id) {
            arc.current_progress = guidance.arc_progress_assessment.progress_percentage;

            // Mark milestone as achieved if progress indicates it
            for milestone in &mut arc.milestones {
                if !milestone.achieved
                    && guidance
                        .arc_progress_assessment
                        .current_milestone
                        .contains(&milestone.description)
                {
                    milestone.achieved = true;
                    milestone.achievement_date = Some(time::OffsetDateTime::now_utc());
                }
            }
        }

        Ok(())
    }

    /// Update story threads when a plot point is added
    async fn update_story_threads_for_plot(&self, plot_point: &PlotPoint) -> Result<()> {
        let mut story_threads = self.story_threads.write().await;

        // Find threads that should include this plot point
        for thread in story_threads.values_mut() {
            if plot_point
                .related_characters
                .iter()
                .any(|c| thread.characters_involved.contains(c))
            {
                thread.related_plot_points.push(plot_point.id.clone());
            }
        }

        Ok(())
    }

    /// Calculate character consistency between states
    async fn calculate_character_consistency(
        &self,
        _current: &CharacterState,
        _new: &CharacterState,
    ) -> f32 {
        // Simplified consistency check
        // In practice, would compare motivations, emotional state changes, etc.
        0.8
    }

    /// Update pacing state based on event impact
    async fn update_pacing_state(&self, event_impact: f32) {
        let mut narrative_state = self.narrative_state.write().await;

        narrative_state.pacing.recent_events_intensity =
            (narrative_state.pacing.recent_events_intensity * 0.7) + (event_impact * 0.3);

        narrative_state.pacing.time_since_major_event = 0; // Reset for major events

        // Adjust pacing type based on intensity
        narrative_state.pacing.current_pace = match narrative_state.pacing.recent_events_intensity {
            x if x < 0.3 => PaceType::Slow,
            x if x < 0.6 => PaceType::Steady,
            x if x < 0.8 => PaceType::Building,
            x if x < 0.95 => PaceType::Climactic,
            _ => PaceType::Resolving,
        };
    }

    /// Process consequences of narrative events
    async fn process_event_consequences(&self, event: &NarrativeEvent) -> Result<()> {
        // Update affected characters
        for participant in &event.participants {
            if let Some(character_state) = self
                .narrative_state
                .read()
                .await
                .character_states
                .get(participant)
            {
                let mut new_state = character_state.clone();
                new_state.last_significant_event = Some(event.id.clone());

                // This would trigger more sophisticated consequence processing
                self.update_character_state(participant.clone(), new_state)
                    .await?;
            }
        }

        Ok(())
    }

    /// Calculate overall narrative coherence health
    async fn calculate_overall_coherence(&self) -> f32 {
        // Simplified coherence calculation
        // In practice, would analyze consistency across all narrative elements
        0.85
    }
}

// Supporting types for API responses

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoherenceAnalysis {
    pub coherence_score: f32,
    pub character_consistency: f32,
    pub world_consistency: f32,
    pub temporal_consistency: f32,
    pub issues: Vec<CoherenceIssue>,
    pub strengths: Vec<String>,
    pub narrative_impact: f32,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceIssue {
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadWeavingResult {
    pub thread_connections: Vec<ThreadConnection>,
    pub new_threads: Vec<NewThread>,
    pub thread_conflicts: Vec<ThreadConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadConnection {
    pub thread_id: String,
    pub connected_plots: Vec<String>,
    pub connection_strength: f32,
    pub narrative_tension: f32,
    pub resolution_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewThread {
    pub title: String,
    pub theme: String,
    pub plot_points: Vec<String>,
    pub estimated_duration: String,
    pub climax_potential: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadConflict {
    pub thread1: String,
    pub thread2: String,
    pub conflict_type: String,
    pub resolution_needed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterGuidance {
    pub arc_progress_assessment: ArcProgressAssessment,
    pub suggested_developments: Vec<SuggestedDevelopment>,
    pub consistency_concerns: Vec<ConsistencyConcern>,
    pub relationship_dynamics: RelationshipDynamics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcProgressAssessment {
    pub current_milestone: String,
    pub progress_percentage: f32,
    pub next_development_needed: String,
    pub timeline_appropriateness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedDevelopment {
    pub development_type: String,
    pub description: String,
    pub narrative_timing: String,
    pub impact_on_relationships: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyConcern {
    pub concern: String,
    pub severity: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDynamics {
    pub strongest_bond: String,
    pub most_tension: String,
    pub development_opportunities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeSummary {
    pub main_storyline: Option<String>,
    pub active_plot_count: usize,
    pub character_count: usize,
    pub story_thread_count: usize,
    pub character_arc_count: usize,
    pub narrative_tone: NarrativeTone,
    pub pacing_state: PacingState,
    pub coherence_health: f32,
}

// Tests commented out pending MockLlmClient and MockRagPipeline implementation
// in astraweave-llm and astraweave-rag crates
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_type_creation() {
        let plot_point = PlotPoint {
            id: Uuid::new_v4().to_string(),
            description: "Test plot point".to_string(),
            plot_type: PlotType::MainQuest,
            importance: 0.8,
            status: PlotStatus::Active,
            related_characters: vec!["character1".to_string()],
            prerequisites: Vec::new(),
            consequences: Vec::new(),
            introduced_at: time::OffsetDateTime::now_utc(),
            deadline: None,
        };

        assert_eq!(plot_point.plot_type, PlotType::MainQuest);
    }
}
