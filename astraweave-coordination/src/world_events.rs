use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
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

use crate::agent::{EventSeverity, WorldEvent};

/// LLM-powered world event generation system
#[allow(dead_code)] // Fields reserved for RAG-enhanced event generation pipeline
pub struct WorldEventGenerator {
    llm_client: Arc<dyn LlmClient>,
    rag_pipeline: Arc<RagPipeline>,
    conversation_history: Arc<RwLock<ConversationHistory>>,
    prompt_library: Arc<RwLock<PromptLibrary>>,
    config: EventGenerationConfig,
    event_templates: Arc<RwLock<HashMap<String, EventTemplate>>>,
    world_state: Arc<RwLock<WorldState>>,
    event_history: Arc<RwLock<Vec<GeneratedEvent>>>,
    active_storylines: Arc<RwLock<Vec<Storyline>>>,
}

/// Configuration for event generation
#[derive(Debug, Clone)]
pub struct EventGenerationConfig {
    pub generation_interval_ms: u64,
    pub max_concurrent_events: usize,
    pub event_severity_distribution: SeverityDistribution,
    pub enable_storyline_continuity: bool,
    pub context_window_size: usize,
    pub creativity_factor: f32,
    pub world_coherence_weight: f32,
}

/// Distribution of event severities
#[derive(Debug, Clone)]
pub struct SeverityDistribution {
    pub trivial: f32,
    pub minor: f32,
    pub moderate: f32,
    pub major: f32,
    pub critical: f32,
}

impl Default for EventGenerationConfig {
    fn default() -> Self {
        Self {
            generation_interval_ms: 30000, // 30 seconds
            max_concurrent_events: 5,
            event_severity_distribution: SeverityDistribution {
                trivial: 0.4,
                minor: 0.3,
                moderate: 0.2,
                major: 0.08,
                critical: 0.02,
            },
            enable_storyline_continuity: true,
            context_window_size: 2048,
            creativity_factor: 0.7,
            world_coherence_weight: 0.8,
        }
    }
}

/// Template for generating events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTemplate {
    pub id: String,
    pub name: String,
    pub category: String,
    pub base_severity: EventSeverity,
    pub triggers: Vec<EventTrigger>,
    pub requirements: EventRequirements,
    pub outcomes: Vec<PossibleOutcome>,
    pub narrative_hooks: Vec<String>,
}

/// Trigger conditions for events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    pub trigger_type: TriggerType,
    pub condition: String,
    pub probability: f32,
    pub cooldown_hours: u32,
}

/// Types of event triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TriggerType {
    Time,         // Time-based triggers
    PlayerAction, // Based on player behavior
    WorldState,   // Based on world conditions
    Random,       // Random occurrence
    Storyline,    // Part of ongoing storyline
    Cascade,      // Triggered by other events
}

/// Requirements for an event to occur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRequirements {
    pub min_player_level: Option<u32>,
    pub required_locations: Vec<String>,
    pub required_npcs: Vec<String>,
    pub world_state_conditions: HashMap<String, serde_json::Value>,
    pub exclude_recent_events: Vec<String>, // Event types to avoid repeating
}

/// Possible outcome of an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossibleOutcome {
    pub description: String,
    pub probability: f32,
    pub world_state_changes: HashMap<String, serde_json::Value>,
    pub spawned_entities: Vec<EntitySpawn>,
    pub unlocked_content: Vec<String>,
    pub follow_up_events: Vec<String>, // Event IDs that might follow
}

/// Entity to spawn as part of event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySpawn {
    pub entity_type: String,
    pub count: u32,
    pub location: String,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Current state of the world for event generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub current_time: DateTime<Utc>,
    pub active_locations: Vec<String>,
    pub player_data: PlayerData,
    pub npc_states: HashMap<String, NpcState>,
    pub faction_relations: HashMap<String, HashMap<String, f32>>,
    pub resource_levels: HashMap<String, f32>,
    pub ongoing_events: Vec<String>, // Event IDs
    pub world_flags: HashMap<String, bool>,
    pub weather: WeatherState,
    pub economic_state: EconomicState,
}

/// Player data for event generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub level: u32,
    pub location: String,
    pub recent_actions: Vec<String>,
    pub reputation: HashMap<String, f32>, // Faction -> reputation
    pub achievements: Vec<String>,
    pub active_quests: Vec<String>,
}

/// NPC state for event generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub location: String,
    pub health: f32,
    pub mood: f32,
    pub busy: bool,
    pub relationships: HashMap<String, f32>,
    pub current_activity: Option<String>,
}

/// Weather state affecting events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherState {
    pub condition: String,
    pub intensity: f32,
    pub duration_hours: u32,
    pub effects: Vec<String>,
}

/// Economic state for trade/economic events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicState {
    pub market_prices: HashMap<String, f32>,
    pub supply_demand: HashMap<String, (f32, f32)>,
    pub trade_routes_status: HashMap<String, String>,
    pub economic_trend: f32, // -1.0 (recession) to 1.0 (boom)
}

/// Generated event with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedEvent {
    pub event: WorldEvent,
    pub generation_context: EventGenerationContext,
    pub outcome: Option<EventOutcome>,
    pub related_events: Vec<String>, // Event IDs
    pub storyline_id: Option<String>,
}

/// Context used to generate the event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventGenerationContext {
    pub template_used: Option<String>,
    pub trigger_type: TriggerType,
    pub world_state_snapshot: String, // Serialized state
    pub generation_reasoning: String,
    pub coherence_score: f32,
    pub generated_at: DateTime<Utc>,
}

/// Outcome of an executed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOutcome {
    pub success: bool,
    pub participant_reactions: HashMap<String, String>,
    pub world_state_changes: HashMap<String, serde_json::Value>,
    pub spawned_entities: Vec<String>, // Entity IDs
    pub narrative_consequences: Vec<String>,
    pub follow_up_events_triggered: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

/// Ongoing storyline connecting multiple events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyline {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub events: Vec<String>, // Event IDs in chronological order
    pub current_phase: u32,
    pub total_phases: u32,
    pub participants: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub estimated_end_time: Option<DateTime<Utc>>,
    pub status: StorylineStatus,
    pub branching_points: Vec<BranchingPoint>,
}

/// Status of a storyline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum StorylineStatus {
    Planning,
    Active,
    Paused,
    Completed,
    Abandoned,
}

/// Point where storyline can branch based on outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchingPoint {
    pub phase: u32,
    pub condition: String,
    pub branches: Vec<StorylineBranch>,
}

/// Branch in a storyline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorylineBranch {
    pub id: String,
    pub description: String,
    pub events: Vec<String>,
    pub consequences: Vec<String>,
}

impl WorldEventGenerator {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        rag_pipeline: Arc<RagPipeline>,
        config: EventGenerationConfig,
    ) -> Result<Self> {
        let context_config = ContextConfig {
            max_tokens: config.context_window_size,
            ..Default::default()
        };
        let conversation_history = Arc::new(RwLock::new(ConversationHistory::new(context_config)));

        let mut prompt_library = PromptLibrary::new();

        // Load event generation prompts
        prompt_library.add_template("event_generation", PromptTemplate::new("event_generation".to_string(),
            r#"
You are a world event generator for a dynamic fantasy game. Create engaging world events based on current game state and player activities.

World State:
- Current Time: {{current_time}}
- Player Level: {{player_level}}
- Player Location: {{player_location}}
- Recent Player Actions: {{recent_actions}}
- Active Locations: {{active_locations}}
- Ongoing Events: {{ongoing_events}}
- Weather: {{weather}}
- Economic State: {{economic_state}}

Event History: {{event_history}}

Faction Relations: {{faction_relations}}

Generate a world event in JSON format:
{
    "event_type": "merchant_caravan_attack",
    "description": "A merchant caravan is under attack by bandits near the forest road",
    "location": "forest_road_east",
    "participants": ["merchant_caravan", "bandit_group"],
    "severity": "Moderate",
    "event_data": {
        "caravan_size": 5,
        "bandit_count": 8,
        "valuable_cargo": true,
        "rescue_time_limit": 300
    },
    "narrative_context": "This attack is part of increased bandit activity due to the recent economic downturn"
}

Guidelines:
1. Events should feel natural and connected to world state
2. Consider player level and location for relevance
3. Create opportunities for player engagement
4. Maintain narrative coherence
5. Balance challenge with player capability
6. Include meaningful consequences
            "#.trim().to_string()
        ));

        prompt_library.add_template("storyline_generation", PromptTemplate::new("storyline_generation".to_string(),
            r#"
You are creating a multi-event storyline for a fantasy game. Design a coherent narrative arc with connected events.

Context:
- World State: {{world_state}}
- Player Data: {{player_data}}
- Recent Events: {{recent_events}}
- Available NPCs: {{available_npcs}}

Generate a storyline in JSON format:
{
    "title": "The Merchant's Conspiracy",
    "description": "A series of events revealing corruption in the merchant guild",
    "category": "Political Intrigue",
    "total_phases": 4,
    "events": [
        {
            "phase": 1,
            "event_type": "suspicious_meeting",
            "description": "Player witnesses a secret meeting between merchants",
            "trigger": "player_enters_market_district_at_night"
        },
        {
            "phase": 2,
            "event_type": "investigation_opportunity",
            "description": "Clues lead to merchant warehouse investigation",
            "trigger": "player_investigates_meeting"
        }
    ],
    "branching_points": [
        {
            "phase": 2,
            "condition": "player_discovers_evidence",
            "branches": [
                {
                    "description": "Expose conspiracy publicly",
                    "events": ["public_trial", "merchant_guild_reform"]
                },
                {
                    "description": "Blackmail merchants privately",
                    "events": ["secret_deal", "underground_influence"]
                }
            ]
        }
    ]
}

Create engaging, branching narratives that respond to player choices.
            "#.trim().to_string()
        ));

        prompt_library.add_template("event_coherence_check", PromptTemplate::new("event_coherence_check".to_string(),
            r#"
You are validating world event coherence. Check if the proposed event makes sense in the current game world.

Proposed Event: {{proposed_event}}

World Context:
- Current State: {{world_state}}
- Recent Events: {{recent_events}}
- Active Storylines: {{active_storylines}}

Provide coherence analysis in JSON format:
{
    "is_coherent": true,
    "coherence_score": 0.85,
    "issues": [
        {
            "type": "logical",
            "severity": "minor",
            "description": "Event location conflicts with recent destruction",
            "suggestion": "Move event to adjacent area"
        }
    ],
    "strengths": [
        "Well connected to player actions",
        "Appropriate for current world state"
    ],
    "recommendations": [
        "Add weather effects to enhance immersion",
        "Include faction response opportunities"
    ]
}

Focus on narrative consistency, logical causation, and player engagement opportunities.
            "#.trim().to_string()
        ));

        let world_state = WorldState {
            current_time: Utc::now(),
            active_locations: vec!["town_square".to_string(), "forest_path".to_string()],
            player_data: PlayerData {
                level: 1,
                location: "town_square".to_string(),
                recent_actions: Vec::new(),
                reputation: HashMap::new(),
                achievements: Vec::new(),
                active_quests: Vec::new(),
            },
            npc_states: HashMap::new(),
            faction_relations: HashMap::new(),
            resource_levels: HashMap::new(),
            ongoing_events: Vec::new(),
            world_flags: HashMap::new(),
            weather: WeatherState {
                condition: "clear".to_string(),
                intensity: 0.0,
                duration_hours: 24,
                effects: Vec::new(),
            },
            economic_state: EconomicState {
                market_prices: HashMap::new(),
                supply_demand: HashMap::new(),
                trade_routes_status: HashMap::new(),
                economic_trend: 0.0,
            },
        };

        Ok(Self {
            llm_client,
            rag_pipeline,
            conversation_history,
            prompt_library: Arc::new(RwLock::new(prompt_library)),
            config,
            event_templates: Arc::new(RwLock::new(HashMap::new())),
            world_state: Arc::new(RwLock::new(world_state)),
            event_history: Arc::new(RwLock::new(Vec::new())),
            active_storylines: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Generate a new world event based on current conditions
    pub async fn generate_event(&self) -> Result<GeneratedEvent> {
        debug!("Generating world event");

        // Check if we should generate an event
        let should_generate = self.should_generate_event().await;
        if !should_generate {
            return Err(anyhow!("Event generation not needed at this time"));
        }

        // Determine event trigger type
        let trigger_type = self.determine_trigger_type().await;

        // Get world context
        let world_context = self.build_world_context().await?;

        // Generate event using LLM
        let event = match &trigger_type {
            TriggerType::Storyline => self.generate_storyline_event(&world_context).await?,
            _ => {
                self.generate_standalone_event(&world_context, trigger_type.clone())
                    .await?
            }
        };

        // Validate event coherence
        let coherence_score = self
            .validate_event_coherence(&event, &world_context)
            .await?;

        if coherence_score < 0.7 {
            warn!(
                "Generated event failed coherence check: {:.2}",
                coherence_score
            );
            return Err(anyhow!("Event coherence too low"));
        }

        // Create generation context
        let generation_context = EventGenerationContext {
            template_used: None,
            trigger_type: trigger_type.clone(),
            world_state_snapshot: serde_json::to_string(&world_context)?,
            generation_reasoning: format!("Generated via {:?} trigger", trigger_type),
            coherence_score,
            generated_at: Utc::now(),
        };

        let generated_event = GeneratedEvent {
            event,
            generation_context,
            outcome: None,
            related_events: Vec::new(),
            storyline_id: None,
        };

        // Store event in history
        self.store_event(&generated_event).await;

        info!(
            "Generated world event: {} ({})",
            generated_event.event.description, generated_event.event.event_type
        );
        Ok(generated_event)
    }

    /// Generate a storyline with multiple connected events
    pub async fn generate_storyline(&self, category: Option<String>) -> Result<Storyline> {
        debug!("Generating storyline");

        let world_context = self.build_world_context().await?;

        // Build storyline prompt
        let storyline_context = HashMap::from([
            (
                "world_state".to_string(),
                serde_json::to_string(&world_context)?,
            ),
            (
                "player_data".to_string(),
                serde_json::to_string(
                    &world_context
                        .get("player_data")
                        .unwrap_or(&serde_json::Value::Null),
                )?,
            ),
            (
                "recent_events".to_string(),
                self.get_recent_events_summary().await,
            ),
            (
                "available_npcs".to_string(),
                self.get_available_npcs().await.join(", "),
            ),
        ]);

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("storyline_generation")?;

        // Convert storyline_context (String->String) into HashMap<String,String> for render_map
        let mut serialized_story: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in &storyline_context {
            serialized_story.insert(k.clone(), v.clone());
        }

        let prompt = template.render_map(&serialized_story)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Storyline generation failed: {}", e))?;

        // Parse storyline data
        #[derive(Deserialize)]
        #[allow(dead_code)] // Fields consumed by serde deserialization; reserved for future storyline branching
        struct StorylineData {
            title: String,
            description: String,
            category: String,
            total_phases: u32,
            events: Vec<serde_json::Value>,
            branching_points: Vec<serde_json::Value>,
        }

        let storyline_data: StorylineData = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse storyline: {}", e))?;

        let storyline = Storyline {
            id: Uuid::new_v4().to_string(),
            title: storyline_data.title,
            description: storyline_data.description,
            category: category.unwrap_or(storyline_data.category),
            events: Vec::new(), // Will be populated as events are generated
            current_phase: 1,
            total_phases: storyline_data.total_phases,
            participants: Vec::new(),
            start_time: Utc::now(),
            estimated_end_time: Some(
                Utc::now() + Duration::hours(storyline_data.total_phases as i64),
            ),
            status: StorylineStatus::Planning,
            branching_points: Vec::new(), // Would be parsed from storyline_data
        };

        // Store storyline
        let mut storylines = self.active_storylines.write().await;
        storylines.push(storyline.clone());

        info!("Generated storyline: {}", storyline.title);
        Ok(storyline)
    }

    /// Update world state for event generation
    pub async fn update_world_state(
        &self,
        updates: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut world_state = self.world_state.write().await;

        for (key, value) in updates {
            match key.as_str() {
                "player_level" => {
                    if let Some(level) = value.as_u64() {
                        world_state.player_data.level = level as u32;
                    }
                }
                "player_location" => {
                    if let Some(location) = value.as_str() {
                        world_state.player_data.location = location.to_string();
                    }
                }
                "weather" => {
                    if let Ok(weather) = serde_json::from_value::<WeatherState>(value) {
                        world_state.weather = weather;
                    }
                }
                _ => {
                    world_state
                        .world_flags
                        .insert(key, value.as_bool().unwrap_or(false));
                }
            }
        }

        world_state.current_time = Utc::now();
        debug!("Updated world state");
        Ok(())
    }

    /// Execute an event and record outcomes
    pub async fn execute_event(&self, event_id: &str, outcome: EventOutcome) -> Result<()> {
        let mut event_history = self.event_history.write().await;

        if let Some(generated_event) = event_history.iter_mut().find(|e| e.event.id == event_id) {
            generated_event.outcome = Some(outcome.clone());

            // Apply world state changes
            self.apply_world_state_changes(&outcome.world_state_changes)
                .await?;

            // Trigger follow-up events
            for follow_up_event_id in &outcome.follow_up_events_triggered {
                debug!("Triggering follow-up event: {}", follow_up_event_id);
                // Would trigger event generation for follow-up events
            }

            // Update storyline progress if part of storyline
            if let Some(storyline_id) = &generated_event.storyline_id {
                self.update_storyline_progress(storyline_id).await?;
            }

            info!(
                "Executed event: {} with {} consequences",
                event_id,
                outcome.narrative_consequences.len()
            );
        }

        Ok(())
    }

    /// Check if event generation should occur
    async fn should_generate_event(&self) -> bool {
        let world_state = self.world_state.read().await;
        let event_history = self.event_history.read().await;

        // Check if we're at the event limit
        if world_state.ongoing_events.len() >= self.config.max_concurrent_events {
            return false;
        }

        // Check time since last event
        if let Some(last_event) = event_history.last() {
            let time_since_last = Utc::now() - last_event.generation_context.generated_at;
            if time_since_last.num_milliseconds() < self.config.generation_interval_ms as i64 {
                return false;
            }
        }

        // Check if player is in a suitable state for events
        !world_state.player_data.recent_actions.is_empty()
    }

    /// Determine the type of trigger for the next event
    async fn determine_trigger_type(&self) -> TriggerType {
        let mut rng = rand::rng();
        let roll: f32 = rng.random();

        // Check for active storylines first
        let storylines = self.active_storylines.read().await;
        if !storylines.is_empty() && self.config.enable_storyline_continuity {
            return TriggerType::Storyline;
        }

        // Weighted random selection
        match roll {
            x if x < 0.3 => TriggerType::Random,
            x if x < 0.6 => TriggerType::PlayerAction,
            x if x < 0.8 => TriggerType::WorldState,
            x if x < 0.95 => TriggerType::Time,
            _ => TriggerType::Cascade,
        }
    }

    /// Build context for event generation
    async fn build_world_context(&self) -> Result<HashMap<String, serde_json::Value>> {
        let world_state = self.world_state.read().await;
        let mut context = HashMap::new();

        context.insert(
            "current_time".to_string(),
            serde_json::to_value(&world_state.current_time)?,
        );
        context.insert(
            "player_level".to_string(),
            serde_json::to_value(world_state.player_data.level)?,
        );
        context.insert(
            "player_location".to_string(),
            serde_json::to_value(&world_state.player_data.location)?,
        );
        context.insert(
            "recent_actions".to_string(),
            serde_json::to_value(&world_state.player_data.recent_actions)?,
        );
        context.insert(
            "active_locations".to_string(),
            serde_json::to_value(&world_state.active_locations)?,
        );
        context.insert(
            "ongoing_events".to_string(),
            serde_json::to_value(&world_state.ongoing_events)?,
        );
        context.insert(
            "weather".to_string(),
            serde_json::to_value(&world_state.weather)?,
        );
        context.insert(
            "economic_state".to_string(),
            serde_json::to_value(&world_state.economic_state)?,
        );
        context.insert(
            "faction_relations".to_string(),
            serde_json::to_value(&world_state.faction_relations)?,
        );

        // Add event history summary
        let event_history = self.event_history.read().await;
        let recent_events: Vec<String> = event_history
            .iter()
            .rev()
            .take(5)
            .map(|e| format!("{}: {}", e.event.event_type, e.event.description))
            .collect();
        context.insert(
            "event_history".to_string(),
            serde_json::to_value(recent_events.join(" | "))?,
        );

        Ok(context)
    }

    /// Generate a standalone event
    async fn generate_standalone_event(
        &self,
        context: &HashMap<String, serde_json::Value>,
        _trigger_type: TriggerType,
    ) -> Result<WorldEvent> {
        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("event_generation")?;

        // Serialize context values to strings for rendering
        let mut serialized_ctx: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in context {
            serialized_ctx.insert(
                k.clone(),
                serde_json::to_string(&v).unwrap_or_else(|_| "null".to_string()),
            );
        }

        let prompt = template.render_map(&serialized_ctx)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Event generation failed: {}", e))?;

        #[derive(Deserialize)]
        #[allow(dead_code)] // Fields consumed by serde deserialization; reserved for future narrative context
        struct EventData {
            event_type: String,
            description: String,
            location: Option<String>,
            participants: Vec<String>,
            severity: String,
            event_data: serde_json::Value,
            narrative_context: Option<String>,
        }

        let event_data: EventData =
            serde_json::from_str(&response).map_err(|e| anyhow!("Failed to parse event: {}", e))?;

        let severity = match event_data.severity.as_str() {
            "Trivial" => EventSeverity::Trivial,
            "Minor" => EventSeverity::Minor,
            "Moderate" => EventSeverity::Moderate,
            "Major" => EventSeverity::Major,
            "Critical" => EventSeverity::Critical,
            _ => EventSeverity::Minor,
        };

        let event = WorldEvent {
            id: Uuid::new_v4().to_string(),
            event_type: event_data.event_type,
            description: event_data.description,
            location: event_data.location,
            participants: event_data.participants,
            event_data: event_data.event_data,
            timestamp: Utc::now(),
            severity,
        };

        Ok(event)
    }

    /// Generate an event as part of a storyline
    async fn generate_storyline_event(
        &self,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<WorldEvent> {
        // For now, generate a standalone event
        // In practice, would select from active storylines and generate next phase
        self.generate_standalone_event(context, TriggerType::Storyline)
            .await
    }

    /// Validate event coherence
    async fn validate_event_coherence(
        &self,
        event: &WorldEvent,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<f32> {
        let coherence_context = HashMap::from([
            ("proposed_event".to_string(), serde_json::to_value(event)?),
            (
                "world_state".to_string(),
                context
                    .get("current_time")
                    .unwrap_or(&serde_json::Value::Null)
                    .clone(),
            ),
            (
                "recent_events".to_string(),
                context
                    .get("event_history")
                    .unwrap_or(&serde_json::Value::Null)
                    .clone(),
            ),
            (
                "active_storylines".to_string(),
                serde_json::to_value(Vec::<String>::new())?,
            ),
        ]);

        let prompt_library = self.prompt_library.read().await;
        let template = prompt_library.get_template("event_coherence_check")?;

        // Serialize coherence_context for rendering
        let mut serialized_coh: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for (k, v) in &coherence_context {
            serialized_coh.insert(
                k.clone(),
                serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()),
            );
        }

        let prompt = template.render_map(&serialized_coh)?;
        drop(prompt_library);

        let response = self
            .llm_client
            .complete(&prompt)
            .await
            .map_err(|e| anyhow!("Coherence check failed: {}", e))?;

        #[derive(Deserialize)]
        struct CoherenceResult {
            coherence_score: f32,
        }

        let result: CoherenceResult = serde_json::from_str(&response)
            .map_err(|e| anyhow!("Failed to parse coherence result: {}", e))?;

        Ok(result.coherence_score)
    }

    /// Store generated event
    async fn store_event(&self, event: &GeneratedEvent) {
        let mut history = self.event_history.write().await;
        history.push(event.clone());

        // Keep history manageable
        if history.len() > 100 {
            history.remove(0);
        }

        // Add to world state ongoing events
        let mut world_state = self.world_state.write().await;
        world_state.ongoing_events.push(event.event.id.clone());
    }

    /// Apply world state changes from event outcome
    async fn apply_world_state_changes(
        &self,
        changes: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut world_state = self.world_state.write().await;

        for (key, value) in changes {
            match key.as_str() {
                "faction_relations" => {
                    if let Ok(relations) = serde_json::from_value::<
                        HashMap<String, HashMap<String, f32>>,
                    >(value.clone())
                    {
                        world_state.faction_relations = relations;
                    }
                }
                "resource_levels" => {
                    if let Ok(resources) =
                        serde_json::from_value::<HashMap<String, f32>>(value.clone())
                    {
                        world_state.resource_levels = resources;
                    }
                }
                _ => {
                    world_state
                        .world_flags
                        .insert(key.clone(), value.as_bool().unwrap_or(false));
                }
            }
        }

        Ok(())
    }

    /// Update storyline progress
    async fn update_storyline_progress(&self, storyline_id: &str) -> Result<()> {
        let mut storylines = self.active_storylines.write().await;

        if let Some(storyline) = storylines.iter_mut().find(|s| s.id == storyline_id) {
            storyline.current_phase += 1;

            if storyline.current_phase > storyline.total_phases {
                storyline.status = StorylineStatus::Completed;
                info!("Completed storyline: {}", storyline.title);
            } else {
                debug!(
                    "Advanced storyline {} to phase {}",
                    storyline.title, storyline.current_phase
                );
            }
        }

        Ok(())
    }

    /// Get summary of recent events
    async fn get_recent_events_summary(&self) -> String {
        let history = self.event_history.read().await;
        history
            .iter()
            .rev()
            .take(3)
            .map(|e| format!("{}: {}", e.event.event_type, e.event.description))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Get list of available NPCs
    async fn get_available_npcs(&self) -> Vec<String> {
        let world_state = self.world_state.read().await;
        world_state.npc_states.keys().cloned().collect()
    }
}

// Add missing import and implementation
use rand::Rng;

// Tests commented out pending MockLlmClient and MockRagPipeline implementation
// in astraweave-llm and astraweave-rag crates
/*
#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_llm::MockLlmClient;
    use astraweave_rag::MockRagPipeline;

    #[tokio::test]
    async fn test_event_generator_creation() {
        let llm_client = Arc::new(MockLlmClient::new());
        let rag_pipeline = Arc::new(MockRagPipeline::new());
        let config = EventGenerationConfig::default();

        let generator = WorldEventGenerator::new(llm_client, rag_pipeline, config);
        assert!(generator.is_ok());
    }

    #[tokio::test]
    async fn test_world_state_updates() {
        let llm_client = Arc::new(MockLlmClient::new());
        let rag_pipeline = Arc::new(MockRagPipeline::new());
        let generator = WorldEventGenerator::new(llm_client, rag_pipeline, EventGenerationConfig::default()).unwrap();

        let mut updates = HashMap::new();
        updates.insert("player_level".to_string(), serde_json::json!(5));
        updates.insert("player_location".to_string(), serde_json::json!("forest"));

        generator.update_world_state(updates).await.unwrap();

        let world_state = generator.world_state.read().await;
        assert_eq!(world_state.player_data.level, 5);
        assert_eq!(world_state.player_data.location, "forest");
    }

    #[test]
    fn test_trigger_type_selection() {
        // Test that trigger type selection works
        // In practice would need to mock random number generation
    }
}
*/
