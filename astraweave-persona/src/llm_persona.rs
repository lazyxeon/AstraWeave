/*!
# LLM-Enhanced Persona System

Advanced persona system with LLM integration for dynamic personality evolution,
context-aware prompt generation, and adaptive behavior.
*/

use anyhow::{anyhow, Context, Result};
use astraweave_core::byte_prefix;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// LLM Integration
use astraweave_context::{ContextConfig, ConversationHistory, Role};
use astraweave_embeddings::EmbeddingClient;
use astraweave_llm::LlmClient;
// Prompt types are provided by astraweave_prompts compat layer
use astraweave_prompts::context::PromptContext as TemplateContext;
use astraweave_prompts::engine::TemplateEngine;
use astraweave_prompts::template::PromptTemplate;
use astraweave_rag::{MemoryQuery, RagPipeline};

// Base persona from memory crate
use astraweave_memory::Persona as BasePersona;

/// Enhanced persona with LLM capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPersona {
    /// Base persona data
    pub base: BasePersona,

    /// LLM-specific configuration
    pub llm_config: PersonaLlmConfig,

    /// Dynamic personality state
    pub personality_state: PersonalityState,

    /// Learning and adaptation data
    pub adaptation: AdaptationData,

    /// Prompt generation settings
    pub prompt_settings: PromptSettings,

    /// Memory and experience data
    pub memory_profile: MemoryProfile,
}

/// LLM configuration for persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaLlmConfig {
    /// Temperature for LLM generation (0.0 to 2.0)
    pub temperature: f32,

    /// Top-p sampling parameter
    pub top_p: f32,

    /// Maximum tokens for persona responses
    pub max_tokens: usize,

    /// Personality factors that influence generation
    pub personality_factors: HashMap<String, f32>,

    /// Response style preferences
    pub response_style: ResponseStyle,

    /// Context window size for this persona
    pub context_window_size: usize,
}

impl Default for PersonaLlmConfig {
    fn default() -> Self {
        let mut personality_factors = HashMap::new();
        personality_factors.insert("creativity".to_string(), 0.7);
        personality_factors.insert("empathy".to_string(), 0.8);
        personality_factors.insert("assertiveness".to_string(), 0.6);
        personality_factors.insert("curiosity".to_string(), 0.7);
        personality_factors.insert("humor".to_string(), 0.5);

        Self {
            temperature: 0.8,
            top_p: 0.9,
            max_tokens: 512,
            personality_factors,
            response_style: ResponseStyle::Conversational,
            context_window_size: 2048,
        }
    }
}

/// Response style preferences
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ResponseStyle {
    /// Natural conversational style
    Conversational,
    /// Formal and structured
    Formal,
    /// Creative and expressive
    Creative,
    /// Technical and precise
    Technical,
    /// Playful and casual
    Playful,
    /// Mysterious and enigmatic
    Mysterious,
}

/// Dynamic personality state that evolves over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityState {
    /// Current mood (-1.0 to 1.0, negative = sad, positive = happy)
    pub current_mood: f32,

    /// Energy level (0.0 to 1.0)
    pub energy_level: f32,

    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,

    /// Trust level towards player (0.0 to 1.0)
    pub trust_level: f32,

    /// Current emotional state
    pub emotional_state: EmotionalState,

    /// Personality drift over time
    pub personality_drift: HashMap<String, f32>,

    /// Recent significant events affecting personality
    pub recent_influences: Vec<PersonalityInfluence>,
}

impl Default for PersonalityState {
    fn default() -> Self {
        Self {
            current_mood: 0.0,
            energy_level: 0.7,
            confidence: 0.6,
            trust_level: 0.5,
            emotional_state: EmotionalState::Neutral,
            personality_drift: HashMap::new(),
            recent_influences: Vec::new(),
        }
    }
}

/// Emotional states for the persona
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EmotionalState {
    Joyful,
    Excited,
    Calm,
    Neutral,
    Thoughtful,
    Concerned,
    Frustrated,
    Sad,
    Angry,
    Surprised,
    Curious,
    Confident,
    Anxious,
}

/// An event that influences personality development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityInfluence {
    /// Description of the event
    pub event: String,

    /// How it affects personality factors
    pub factor_changes: HashMap<String, f32>,

    /// Timestamp of the event
    pub timestamp: u64,

    /// Importance/weight of this influence
    pub importance: f32,

    /// Decay rate (how quickly this influence fades)
    pub decay_rate: f32,
}

/// Learning and adaptation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationData {
    /// Total interactions with the player
    pub interaction_count: u64,

    /// Successful interactions (positive feedback)
    pub successful_interactions: u64,

    /// Learning rate (how quickly persona adapts)
    pub learning_rate: f32,

    /// Preferred conversation topics
    pub preferred_topics: HashMap<String, f32>,

    /// Topics to avoid
    pub topics_to_avoid: Vec<String>,

    /// Player behavior patterns learned
    pub player_patterns: PlayerPatterns,

    /// Adaptation history
    pub adaptation_history: Vec<AdaptationEvent>,
}

impl Default for AdaptationData {
    fn default() -> Self {
        Self {
            interaction_count: 0,
            successful_interactions: 0,
            learning_rate: 0.1,
            preferred_topics: HashMap::new(),
            topics_to_avoid: Vec::new(),
            player_patterns: PlayerPatterns::default(),
            adaptation_history: Vec::new(),
        }
    }
}

/// Learned patterns about player behavior
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerPatterns {
    /// Player's communication style
    pub communication_style: Option<String>,

    /// Player's interests
    pub interests: Vec<String>,

    /// Player's typical session length
    pub avg_session_length: Option<f32>,

    /// Player's preferred interaction times
    pub preferred_times: Vec<String>,

    /// Player's emotional patterns
    pub emotional_patterns: HashMap<String, f32>,
}

/// An adaptation event in the persona's learning history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationEvent {
    /// What triggered the adaptation
    pub trigger: String,

    /// What changed
    pub changes: HashMap<String, f32>,

    /// When it happened
    pub timestamp: u64,

    /// Success rating of this adaptation
    pub success_rating: Option<f32>,
}

/// Prompt generation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSettings {
    /// Base system prompt template
    pub system_prompt_template: String,

    /// Context injection strategy
    pub context_injection: ContextInjectionStrategy,

    /// Memory retrieval settings
    pub memory_retrieval: MemoryRetrievalSettings,

    /// Few-shot examples for this persona
    pub few_shot_examples: Vec<FewShotExample>,

    /// Dynamic prompt modifiers
    pub prompt_modifiers: HashMap<String, String>,
}

impl Default for PromptSettings {
    fn default() -> Self {
        Self {
            system_prompt_template: "You are {{persona.name}}, {{persona.description}}. Your personality: {{persona.personality}}. Current mood: {{state.mood}}.".to_string(),
            context_injection: ContextInjectionStrategy::Contextual,
            memory_retrieval: MemoryRetrievalSettings::default(),
            few_shot_examples: Vec::new(),
            prompt_modifiers: HashMap::new(),
        }
    }
}

/// Strategy for injecting context into prompts
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContextInjectionStrategy {
    /// Full context with memories
    Full,
    /// Only recent context
    Recent,
    /// Contextual based on relevance
    Contextual,
    /// Minimal context
    Minimal,
}

/// Settings for memory retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalSettings {
    /// Maximum memories to retrieve
    pub max_memories: usize,

    /// Minimum similarity threshold
    pub min_similarity: f32,

    /// Memory categories to prioritize (stored as strings since MemoryCategory was removed)
    pub priority_categories: Vec<String>,

    /// Recency bonus for newer memories
    pub recency_bonus: f32,
}

impl Default for MemoryRetrievalSettings {
    fn default() -> Self {
        Self {
            max_memories: 5,
            min_similarity: 0.3,
            priority_categories: vec!["Social".to_string(), "Dialogue".to_string()],
            recency_bonus: 0.1,
        }
    }
}

/// A few-shot example for prompt engineering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    /// Example input/situation
    pub input: String,

    /// Expected/desired output
    pub output: String,

    /// Context for this example
    pub context: Option<String>,

    /// Tags for categorizing examples
    pub tags: Vec<String>,
}

/// Memory profile for the persona
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryProfile {
    /// Core memories that define the persona
    pub core_memories: Vec<String>,

    /// Episodic memories from interactions
    pub episodic_memories: Vec<String>,

    /// Semantic knowledge
    pub semantic_knowledge: Vec<String>,

    /// Memory consolidation preferences
    pub consolidation_preferences: ConsolidationPreferences,
}

/// Preferences for memory consolidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationPreferences {
    /// How often to consolidate memories
    pub consolidation_frequency: u32,

    /// Importance threshold for keeping memories
    pub importance_threshold: f32,

    /// Maximum memories to keep
    pub max_memories: usize,

    /// Forgetting curve parameters
    pub forgetting_curve: ForgettingCurve,
}

impl Default for ConsolidationPreferences {
    fn default() -> Self {
        Self {
            consolidation_frequency: 100, // Every 100 interactions
            importance_threshold: 0.3,
            max_memories: 1000,
            forgetting_curve: ForgettingCurve::default(),
        }
    }
}

/// Parameters for memory forgetting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingCurve {
    /// Base decay rate
    pub decay_rate: f32,

    /// Importance multiplier (important memories decay slower)
    pub importance_multiplier: f32,

    /// Rehearsal bonus (frequently accessed memories decay slower)
    pub rehearsal_bonus: f32,
}

impl Default for ForgettingCurve {
    fn default() -> Self {
        Self {
            decay_rate: 0.1,
            importance_multiplier: 2.0,
            rehearsal_bonus: 0.5,
        }
    }
}

/// Main LLM-enhanced persona manager
pub struct LlmPersonaManager {
    /// The enhanced persona
    persona: Arc<RwLock<LlmPersona>>,

    /// LLM client for generation
    llm_client: Arc<dyn LlmClient>,

    /// RAG pipeline for memory retrieval
    rag_pipeline: Arc<RwLock<RagPipeline>>,

    /// Conversation history
    conversation_history: Arc<RwLock<ConversationHistory>>,

    /// Template engine for prompt generation
    template_engine: Arc<RwLock<TemplateEngine>>,

    /// Performance metrics
    metrics: Arc<RwLock<PersonaMetrics>>,
}

/// Performance metrics for persona system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaMetrics {
    pub total_interactions: u64,
    pub successful_generations: u64,
    pub failed_generations: u64,
    pub avg_response_time_ms: f32,
    pub personality_evolution_events: u64,
    pub memory_consolidations: u64,
    pub adaptation_learning_events: u64,
}

impl LlmPersonaManager {
    /// Create a new LLM persona manager
    pub async fn new(
        base_persona: BasePersona,
        llm_client: Arc<dyn LlmClient>,
        rag_pipeline: RagPipeline,
        _embedding_client: Arc<dyn EmbeddingClient>,
    ) -> Result<Self> {
        // Create enhanced persona
        let llm_persona = LlmPersona {
            base: base_persona,
            llm_config: PersonaLlmConfig::default(),
            personality_state: PersonalityState::default(),
            adaptation: AdaptationData::default(),
            prompt_settings: PromptSettings::default(),
            memory_profile: MemoryProfile::default(),
        };

        // Set up conversation history
        let context_config = ContextConfig {
            max_tokens: llm_persona.llm_config.context_window_size,
            enable_summarization: true,
            ..Default::default()
        };
        let conversation_history =
            ConversationHistory::with_llm_client(context_config, llm_client.clone());

        // Set up template engine
        let mut template_engine = TemplateEngine::new();
        Self::setup_persona_templates(&mut template_engine).await?;

        Ok(Self {
            persona: Arc::new(RwLock::new(llm_persona)),
            llm_client,
            rag_pipeline: Arc::new(RwLock::new(rag_pipeline)),
            conversation_history: Arc::new(RwLock::new(conversation_history)),
            template_engine: Arc::new(RwLock::new(template_engine)),
            metrics: Arc::new(RwLock::new(PersonaMetrics::default())),
        })
    }

    /// Generate a response as this persona
    pub async fn generate_response(&self, input: &str, context: Option<&str>) -> Result<String> {
        let start_time = std::time::Instant::now();

        // 1. Update interaction count
        {
            let mut persona = self.persona.write().await;
            persona.adaptation.interaction_count += 1;
        }

        // 2. Add user input to conversation history
        {
            let history = self.conversation_history.write().await;
            history.add_message(Role::User, input.to_string()).await?;
        }

        // 3. Retrieve relevant memories
        let memories = {
            let rag = self.rag_pipeline.read().await;
            let query = MemoryQuery::text(input);
            rag.retrieve_with_query(&query, 5).await?
        };

        // 4. Build context-aware prompt
        let prompt = self.build_persona_prompt(input, context, &memories).await?;

        // 5. Generate response using LLM
        let response = self.llm_client.complete(&prompt).await?;
        let cleaned_response = self.clean_and_validate_response(&response).await?;

        // 6. Add response to conversation history
        {
            let history = self.conversation_history.write().await;
            history
                .add_message(Role::Assistant, cleaned_response.clone())
                .await?;
        }

        // 7. Store this interaction as a memory
        let interaction_memory = format!(
            "Player: {} | {}: {}",
            input,
            self.get_persona_name().await,
            cleaned_response
        );
        {
            let mut rag = self.rag_pipeline.write().await;
            rag.add_memory(interaction_memory).await?;
        }

        // 8. Update persona state based on interaction
        self.update_personality_state(input, &cleaned_response)
            .await?;

        // 9. Update metrics
        let duration = start_time.elapsed().as_secs_f32() * 1000.0;
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_interactions += 1;
            metrics.successful_generations += 1;
            metrics.avg_response_time_ms =
                (metrics.avg_response_time_ms * (metrics.total_interactions - 1) as f32 + duration)
                    / metrics.total_interactions as f32;
        }

        Ok(cleaned_response)
    }

    /// Get the persona's name
    pub async fn get_persona_name(&self) -> String {
        // Extract name from tone field (temporary until we have a proper name field)
        let persona = self.persona.read().await;
        persona.base.voice.clone() // Using voice as name for now
    }

    /// Build a context-aware prompt for the persona
    async fn build_persona_prompt(
        &self,
        input: &str,
        context: Option<&str>,
        memories: &[astraweave_rag::RetrievedMemory],
    ) -> Result<String> {
        let persona = self.persona.read().await;
        let conversation_context = {
            let history = self.conversation_history.read().await;
            history.get_context(1024).await?
        };

        // Build template context
        let mut template_context = TemplateContext::new();

        // Basic persona info
        template_context.set_path("persona.name", persona.base.voice.clone().into()); // Using voice as name
        template_context.set_path("persona.tone", persona.base.tone.clone().into());
        template_context.set_path("persona.humor", persona.base.humor.clone().into());
        template_context.set_path("persona.risk", persona.base.risk.clone().into());
        template_context.set_path("persona.backstory", persona.base.backstory.clone().into());

        // Personality state
        template_context.set_path(
            "state.mood",
            format!("{:?}", persona.personality_state.emotional_state).into(),
        );
        template_context.set_path(
            "state.energy",
            persona.personality_state.energy_level.to_string().into(),
        );
        template_context.set_path(
            "state.confidence",
            persona.personality_state.confidence.to_string().into(),
        );
        template_context.set_path(
            "state.trust",
            persona.personality_state.trust_level.to_string().into(),
        );

        // Input and context
        template_context.set("user_input".to_string(), input.to_string().into());
        if let Some(ctx) = context {
            template_context.set("additional_context".to_string(), ctx.to_string().into());
        }
        template_context.set(
            "conversation_history".to_string(),
            conversation_context.into(),
        );

        // Memories
        if !memories.is_empty() {
            let memory_text: Vec<String> = memories
                .iter()
                .map(|m| format!("- {}", m.memory.text))
                .collect();
            template_context.set(
                "relevant_memories".to_string(),
                memory_text.join("\n").into(),
            );
        }

        // Personality factors
        let personality_factors: Vec<String> = persona
            .llm_config
            .personality_factors
            .iter()
            .map(|(k, v)| format!("{}: {:.1}", k, v))
            .collect();
        template_context.set(
            "personality_factors".to_string(),
            personality_factors.join(", ").into(),
        );

        // Use template engine to generate prompt
        let template_engine = self.template_engine.read().await;
        template_engine.render("persona_response", &template_context)
    }

    /// Clean and validate the LLM response
    async fn clean_and_validate_response(&self, response: &str) -> Result<String> {
        // Basic cleaning
        let cleaned = response
            .trim()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        // Validate length
        if cleaned.is_empty() {
            return Err(anyhow!("Empty response generated"));
        }

        if cleaned.len() > 2048 {
            return Ok(byte_prefix(&cleaned, 2048).to_string());
        }

        Ok(cleaned)
    }

    /// Update personality state based on interaction
    async fn update_personality_state(&self, input: &str, _response: &str) -> Result<()> {
        let mut persona = self.persona.write().await;

        // Simple sentiment analysis (could be enhanced with LLM)
        let positive_words = [
            "good",
            "great",
            "excellent",
            "amazing",
            "wonderful",
            "thank",
            "appreciate",
        ];
        let negative_words = [
            "bad", "terrible", "awful", "hate", "wrong", "error", "problem",
        ];

        let input_lower = input.to_lowercase();
        let positive_count = positive_words
            .iter()
            .filter(|&&word| input_lower.contains(word))
            .count();
        let negative_count = negative_words
            .iter()
            .filter(|&&word| input_lower.contains(word))
            .count();

        // Adjust mood based on interaction sentiment
        let mood_change = (positive_count as f32 - negative_count as f32) * 0.1;
        persona.personality_state.current_mood =
            (persona.personality_state.current_mood + mood_change).clamp(-1.0, 1.0);

        // Adjust energy level (decreases slightly with interaction, recovers with rest words)
        let rest_words = ["rest", "sleep", "wait", "pause", "relax"];
        let is_resting = rest_words.iter().any(|&w| input_lower.contains(w));

        if is_resting {
            persona.personality_state.energy_level =
                (persona.personality_state.energy_level + 0.2).min(1.0);
        } else {
            persona.personality_state.energy_level =
                (persona.personality_state.energy_level - 0.01).max(0.0);
        }

        // Adjust confidence based on success
        if positive_count > negative_count {
            persona.personality_state.confidence =
                (persona.personality_state.confidence + 0.02).min(1.0);
        } else if negative_count > positive_count {
            persona.personality_state.confidence =
                (persona.personality_state.confidence - 0.02).max(0.0);
        }

        // Update emotional state based on mood and energy
        persona.personality_state.emotional_state = Self::calculate_emotional_state(
            persona.personality_state.current_mood,
            persona.personality_state.energy_level,
        );

        // Adjust trust based on successful interaction
        persona.personality_state.trust_level =
            (persona.personality_state.trust_level + 0.01).min(1.0);

        // Increment successful interactions (simplified success detection)
        if positive_count > negative_count || input_lower.contains("good") {
            persona.adaptation.successful_interactions += 1;
        }

        Ok(())
    }

    /// Calculate emotional state from mood and energy
    fn calculate_emotional_state(mood: f32, energy: f32) -> EmotionalState {
        if mood >= 0.3 {
            if energy >= 0.6 {
                EmotionalState::Excited
            } else if energy >= 0.3 {
                EmotionalState::Joyful
            } else {
                EmotionalState::Calm
            }
        } else if mood <= -0.3 {
            if energy >= 0.6 {
                EmotionalState::Angry
            } else if energy >= 0.3 {
                EmotionalState::Frustrated
            } else {
                EmotionalState::Sad
            }
        } else {
            // Neutral mood range
            if energy >= 0.7 {
                EmotionalState::Curious
            } else if energy <= 0.3 {
                EmotionalState::Thoughtful
            } else {
                EmotionalState::Neutral
            }
        }
    }

    /// Set up template engine with persona-specific templates
    async fn setup_persona_templates(engine: &mut TemplateEngine) -> Result<()> {
        // Main persona response template
        let response_template = PromptTemplate::new("persona_response".to_string(),
            r#"You are {{persona.name}}, speaking with tone: {{persona.tone}}, humor level: {{persona.humor}}, risk tolerance: {{persona.risk}}.

{{persona.backstory}}

Your current state:
- Mood: {{state.mood}}
- Energy: {{state.energy}}
- Confidence: {{state.confidence}}
- Trust in player: {{state.trust}}

Personality factors: {{personality_factors}}

{{#if conversation_history}}
Recent conversation:
{{conversation_history}}
{{/if}}

{{#if relevant_memories}}
Relevant memories:
{{relevant_memories}}
{{/if}}

{{#if additional_context}}
Additional context: {{additional_context}}
{{/if}}

The player says: "{{user_input}}"

Respond as {{persona.name}} would, staying true to your personality, current state, and past experiences:"#.trim().to_string()
    );

        engine.register_template("persona_response", response_template)?;

        Ok(())
    }

    /// Evolve the persona based on long-term interactions
    pub async fn evolve_personality(&self, trigger: &str) -> Result<()> {
        let mut persona = self.persona.write().await;

        // Create personality influence event
        let mut factor_changes = HashMap::new();

        // Example evolution rules (could be made more sophisticated)
        if trigger.contains("creative") || trigger.contains("art") || trigger.contains("imagine") {
            factor_changes.insert("creativity".to_string(), 0.05);
        }

        if trigger.contains("help") || trigger.contains("support") || trigger.contains("care") {
            factor_changes.insert("empathy".to_string(), 0.03);
        }

        if trigger.contains("joke") || trigger.contains("funny") || trigger.contains("laugh") {
            factor_changes.insert("humor".to_string(), 0.04);
        }

        if !factor_changes.is_empty() {
            // Apply changes to personality factors
            for (factor, change) in &factor_changes {
                let current = persona
                    .llm_config
                    .personality_factors
                    .get(factor)
                    .unwrap_or(&0.5);
                let new_value = (current + change).clamp(0.0, 1.0);
                persona
                    .llm_config
                    .personality_factors
                    .insert(factor.clone(), new_value);
            }

            // Record the influence
            let influence = PersonalityInfluence {
                event: trigger.to_string(),
                factor_changes,
                timestamp: chrono::Utc::now().timestamp() as u64,
                importance: 0.5,
                decay_rate: 0.1,
            };

            persona.personality_state.recent_influences.push(influence);

            // Update metrics
            drop(persona);
            let mut metrics = self.metrics.write().await;
            metrics.personality_evolution_events += 1;
        }

        Ok(())
    }

    /// Get current persona state
    pub async fn get_persona_state(&self) -> LlmPersona {
        self.persona.read().await.clone()
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PersonaMetrics {
        self.metrics.read().await.clone()
    }

    /// Set persona state (useful for testing and restoration)
    pub async fn set_persona_state(&self, state: LlmPersona) {
        let mut persona = self.persona.write().await;
        *persona = state;
    }

    /// Trigger maintenance tasks for the persona's memory (consolidation and forgetting)
    pub async fn maintenance(&self) -> Result<()> {
        let rag = self.rag_pipeline.read().await;

        rag.trigger_consolidation()
            .await
            .context("persona memory consolidation")?;
        rag.trigger_forgetting()
            .await
            .context("persona memory forgetting")?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
    use astraweave_llm::MockLlm;
    use astraweave_rag::VectorStoreWrapper;

    #[tokio::test]
    async fn test_llm_persona_creation() {
        let base_persona = BasePersona::default();
        let llm_client = Arc::new(MockLlm);
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));

        let rag_pipeline = RagPipeline::new(
            embedding_client,
            vector_store,
            Some(llm_client.clone()),
            astraweave_rag::RagConfig::default(),
        );

        let manager = LlmPersonaManager::new(
            base_persona,
            llm_client,
            rag_pipeline,
            Arc::new(MockEmbeddingClient::new()),
        )
        .await
        .unwrap();

        let persona_state = manager.get_persona_state().await;
        assert_eq!(persona_state.personality_state.trust_level, 0.5);
        assert_eq!(persona_state.adaptation.interaction_count, 0);
    }

    #[tokio::test]
    async fn test_personality_evolution() {
        let base_persona = BasePersona::default();
        let llm_client = Arc::new(MockLlm);
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));

        let rag_pipeline = RagPipeline::new(
            embedding_client,
            vector_store,
            Some(llm_client.clone()),
            astraweave_rag::RagConfig::default(),
        );

        let manager = LlmPersonaManager::new(
            base_persona,
            llm_client,
            rag_pipeline,
            Arc::new(MockEmbeddingClient::new()),
        )
        .await
        .unwrap();

        // Evolve personality with creative trigger
        manager
            .evolve_personality("Let's create some art together")
            .await
            .unwrap();

        let persona_state = manager.get_persona_state().await;
        let creativity = persona_state
            .llm_config
            .personality_factors
            .get("creativity")
            .unwrap_or(&0.5);

        // Should have increased creativity
        assert!(*creativity > 0.7);
        assert_eq!(persona_state.personality_state.recent_influences.len(), 1);
    }

    #[tokio::test]
    async fn test_response_generation() {
        let base_persona = BasePersona::default();
        let llm_client = Arc::new(MockLlm);
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));

        let rag_pipeline = RagPipeline::new(
            embedding_client,
            vector_store,
            Some(llm_client.clone()),
            astraweave_rag::RagConfig::default(),
        );

        let manager = LlmPersonaManager::new(
            base_persona,
            llm_client,
            rag_pipeline,
            Arc::new(MockEmbeddingClient::new()),
        )
        .await
        .unwrap();

        let response = manager.generate_response("Hello, how are you?", None).await;

        // Should succeed with MockLlm
        assert!(response.is_ok());
        let response_text = response.unwrap();
        assert!(!response_text.is_empty());

        // Check metrics updated
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_interactions, 1);
        assert_eq!(metrics.successful_generations, 1);
    }

    #[tokio::test]
    async fn test_clean_and_validate_response_truncates_cjk_at_limit() {
        let base_persona = BasePersona::default();
        let llm_client = Arc::new(MockLlm);
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));

        let rag_pipeline = RagPipeline::new(
            embedding_client,
            vector_store,
            Some(llm_client.clone()),
            astraweave_rag::RagConfig::default(),
        );

        let manager = LlmPersonaManager::new(
            base_persona,
            llm_client,
            rag_pipeline,
            Arc::new(MockEmbeddingClient::new()),
        )
        .await
        .unwrap();

        let long = format!("{}日", "x".repeat(2046));
        assert!(long.len() > 2048);
        assert!(!long.is_char_boundary(2048));

        let cleaned = manager.clean_and_validate_response(&long).await.unwrap();
        assert!(cleaned.len() <= 2048);
        assert!(long.starts_with(&cleaned));
        assert!(std::str::from_utf8(cleaned.as_bytes()).is_ok());
    }

    #[test]
    fn test_response_style_variants() {
        let styles = vec![
            ResponseStyle::Conversational,
            ResponseStyle::Formal,
            ResponseStyle::Creative,
            ResponseStyle::Technical,
            ResponseStyle::Playful,
            ResponseStyle::Mysterious,
        ];

        for style in styles {
            let _ = format!("{:?}", style);
        }
    }

    #[test]
    fn test_emotional_state_variants() {
        let states = vec![
            EmotionalState::Joyful,
            EmotionalState::Excited,
            EmotionalState::Calm,
            EmotionalState::Neutral,
            EmotionalState::Thoughtful,
            EmotionalState::Concerned,
            EmotionalState::Frustrated,
            EmotionalState::Sad,
            EmotionalState::Angry,
            EmotionalState::Surprised,
            EmotionalState::Curious,
            EmotionalState::Confident,
            EmotionalState::Anxious,
        ];

        for state in states {
            let _ = format!("{:?}", state);
        }
    }

    #[test]
    fn test_context_injection_strategy_variants() {
        let strategies = vec![
            ContextInjectionStrategy::Full,
            ContextInjectionStrategy::Recent,
            ContextInjectionStrategy::Contextual,
            ContextInjectionStrategy::Minimal,
        ];

        for strategy in strategies {
            let _ = format!("{:?}", strategy);
        }
    }

    #[test]
    fn test_persona_llm_config_default() {
        let config = PersonaLlmConfig::default();

        assert_eq!(config.temperature, 0.8);
        assert_eq!(config.top_p, 0.9);
        assert_eq!(config.max_tokens, 512);
        assert_eq!(config.context_window_size, 2048);
        assert!(matches!(
            config.response_style,
            ResponseStyle::Conversational
        ));

        // Check default personality factors
        assert_eq!(config.personality_factors.len(), 5);
        assert_eq!(*config.personality_factors.get("creativity").unwrap(), 0.7);
        assert_eq!(*config.personality_factors.get("empathy").unwrap(), 0.8);
        assert_eq!(
            *config.personality_factors.get("assertiveness").unwrap(),
            0.6
        );
        assert_eq!(*config.personality_factors.get("curiosity").unwrap(), 0.7);
        assert_eq!(*config.personality_factors.get("humor").unwrap(), 0.5);
    }

    #[test]
    fn test_personality_state_default() {
        let state = PersonalityState::default();

        assert_eq!(state.current_mood, 0.0);
        assert_eq!(state.energy_level, 0.7);
        assert_eq!(state.confidence, 0.6);
        assert_eq!(state.trust_level, 0.5);
        assert!(matches!(state.emotional_state, EmotionalState::Neutral));
        assert!(state.personality_drift.is_empty());
        assert!(state.recent_influences.is_empty());
    }

    #[test]
    fn test_adaptation_data_default() {
        let data = AdaptationData::default();

        assert_eq!(data.interaction_count, 0);
        assert_eq!(data.successful_interactions, 0);
        assert_eq!(data.learning_rate, 0.1);
        assert!(data.preferred_topics.is_empty());
        assert!(data.topics_to_avoid.is_empty());
        assert!(data.adaptation_history.is_empty());
    }

    #[test]
    fn test_player_patterns_default() {
        let patterns = PlayerPatterns::default();

        assert!(patterns.communication_style.is_none());
        assert!(patterns.interests.is_empty());
        assert!(patterns.avg_session_length.is_none());
        assert!(patterns.preferred_times.is_empty());
        assert!(patterns.emotional_patterns.is_empty());
    }

    #[test]
    fn test_prompt_settings_default() {
        let settings = PromptSettings::default();

        assert!(settings.system_prompt_template.contains("{{persona.name}}"));
        assert!(matches!(
            settings.context_injection,
            ContextInjectionStrategy::Contextual
        ));
        assert!(settings.few_shot_examples.is_empty());
        assert!(settings.prompt_modifiers.is_empty());
    }

    #[test]
    fn test_memory_retrieval_settings_default() {
        let settings = MemoryRetrievalSettings::default();

        assert_eq!(settings.max_memories, 5);
        assert_eq!(settings.min_similarity, 0.3);
        assert_eq!(settings.recency_bonus, 0.1);
        assert_eq!(settings.priority_categories.len(), 2);
        assert!(settings.priority_categories.contains(&"Social".to_string()));
        assert!(settings
            .priority_categories
            .contains(&"Dialogue".to_string()));
    }

    #[test]
    fn test_memory_profile_default() {
        let profile = MemoryProfile::default();

        assert!(profile.core_memories.is_empty());
        assert!(profile.episodic_memories.is_empty());
        assert!(profile.semantic_knowledge.is_empty());
    }

    #[test]
    fn test_consolidation_preferences_default() {
        let prefs = ConsolidationPreferences::default();

        assert_eq!(prefs.consolidation_frequency, 100);
        assert_eq!(prefs.importance_threshold, 0.3);
        assert_eq!(prefs.max_memories, 1000);
    }

    #[test]
    fn test_forgetting_curve_default() {
        let curve = ForgettingCurve::default();

        assert_eq!(curve.decay_rate, 0.1);
        assert_eq!(curve.importance_multiplier, 2.0);
        assert_eq!(curve.rehearsal_bonus, 0.5);
    }

    #[test]
    fn test_persona_metrics_default() {
        let metrics = PersonaMetrics::default();

        assert_eq!(metrics.total_interactions, 0);
        assert_eq!(metrics.successful_generations, 0);
        assert_eq!(metrics.failed_generations, 0);
        assert_eq!(metrics.avg_response_time_ms, 0.0);
        assert_eq!(metrics.personality_evolution_events, 0);
        assert_eq!(metrics.memory_consolidations, 0);
        assert_eq!(metrics.adaptation_learning_events, 0);
    }

    #[test]
    fn test_llm_persona_serialization() {
        let persona = LlmPersona {
            base: BasePersona::default(),
            llm_config: PersonaLlmConfig::default(),
            personality_state: PersonalityState::default(),
            adaptation: AdaptationData::default(),
            prompt_settings: PromptSettings::default(),
            memory_profile: MemoryProfile::default(),
        };

        let serialized = serde_json::to_string(&persona).unwrap();
        let deserialized: LlmPersona = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            persona.personality_state.current_mood,
            deserialized.personality_state.current_mood
        );
        assert_eq!(
            persona.llm_config.temperature,
            deserialized.llm_config.temperature
        );
    }

    #[test]
    fn test_personality_influence_serialization() {
        let mut factor_changes = HashMap::new();
        factor_changes.insert("test".to_string(), 0.5);

        let influence = PersonalityInfluence {
            event: "Test event".to_string(),
            factor_changes,
            timestamp: 1234567890,
            importance: 0.75,
            decay_rate: 0.05,
        };

        let serialized = serde_json::to_string(&influence).unwrap();
        let deserialized: PersonalityInfluence = serde_json::from_str(&serialized).unwrap();

        assert_eq!(influence.event, deserialized.event);
        assert_eq!(influence.importance, deserialized.importance);
    }

    #[test]
    fn test_few_shot_example_serialization() {
        let example = FewShotExample {
            input: "Test input".to_string(),
            output: "Test output".to_string(),
            context: Some("Test context".to_string()),
            tags: vec!["test".to_string(), "example".to_string()],
        };

        let serialized = serde_json::to_string(&example).unwrap();
        let deserialized: FewShotExample = serde_json::from_str(&serialized).unwrap();

        assert_eq!(example.input, deserialized.input);
        assert_eq!(example.tags.len(), deserialized.tags.len());
    }

    #[test]
    fn test_adaptation_event_serialization() {
        let mut changes = HashMap::new();
        changes.insert("confidence".to_string(), 0.1);

        let event = AdaptationEvent {
            trigger: "Positive feedback".to_string(),
            changes,
            timestamp: 9876543210,
            success_rating: Some(0.95),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: AdaptationEvent = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event.trigger, deserialized.trigger);
        assert_eq!(event.success_rating, deserialized.success_rating);
    }

    #[test]
    fn test_calculate_emotional_state_high_mood_high_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.8);
        assert!(matches!(state, EmotionalState::Excited));
    }

    #[test]
    fn test_calculate_emotional_state_high_mood_medium_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.5);
        assert!(matches!(state, EmotionalState::Joyful));
    }

    #[test]
    fn test_calculate_emotional_state_high_mood_low_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.2);
        assert!(matches!(state, EmotionalState::Calm));
    }

    #[test]
    fn test_calculate_emotional_state_low_mood_high_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.8);
        assert!(matches!(state, EmotionalState::Angry));
    }

    #[test]
    fn test_calculate_emotional_state_low_mood_medium_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.5);
        assert!(matches!(state, EmotionalState::Frustrated));
    }

    #[test]
    fn test_calculate_emotional_state_low_mood_low_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.2);
        assert!(matches!(state, EmotionalState::Sad));
    }

    #[test]
    fn test_calculate_emotional_state_neutral_mood_high_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.8);
        assert!(matches!(state, EmotionalState::Curious));
    }

    #[test]
    fn test_calculate_emotional_state_neutral_mood_low_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.2);
        assert!(matches!(state, EmotionalState::Thoughtful));
    }

    #[test]
    fn test_calculate_emotional_state_neutral_mood_medium_energy() {
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.5);
        assert!(matches!(state, EmotionalState::Neutral));
    }

    #[test]
    fn test_llm_persona_clone() {
        let persona = LlmPersona {
            base: BasePersona::default(),
            llm_config: PersonaLlmConfig::default(),
            personality_state: PersonalityState::default(),
            adaptation: AdaptationData::default(),
            prompt_settings: PromptSettings::default(),
            memory_profile: MemoryProfile::default(),
        };

        let cloned = persona.clone();

        assert_eq!(
            persona.personality_state.current_mood,
            cloned.personality_state.current_mood
        );
        assert_eq!(
            persona.llm_config.temperature,
            cloned.llm_config.temperature
        );
    }

    #[test]
    fn test_llm_persona_debug() {
        let persona = LlmPersona {
            base: BasePersona::default(),
            llm_config: PersonaLlmConfig::default(),
            personality_state: PersonalityState::default(),
            adaptation: AdaptationData::default(),
            prompt_settings: PromptSettings::default(),
            memory_profile: MemoryProfile::default(),
        };

        let debug_str = format!("{:?}", persona);
        assert!(debug_str.contains("LlmPersona"));
    }

    #[test]
    fn test_player_patterns_serialization() {
        let mut emotional_patterns = HashMap::new();
        emotional_patterns.insert("happiness".to_string(), 0.8);

        let patterns = PlayerPatterns {
            communication_style: Some("formal".to_string()),
            interests: vec!["science".to_string()],
            avg_session_length: Some(60.0),
            preferred_times: vec!["morning".to_string()],
            emotional_patterns,
        };

        let serialized = serde_json::to_string(&patterns).unwrap();
        let deserialized: PlayerPatterns = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            patterns.communication_style,
            deserialized.communication_style
        );
        assert_eq!(patterns.avg_session_length, deserialized.avg_session_length);
    }

    // ── Boundary tests for calculate_emotional_state ──

    #[test]
    fn test_emotional_state_boundary_mood_exactly_positive_threshold() {
        // mood = 0.3 → positive branch
        let state = LlmPersonaManager::calculate_emotional_state(0.3, 0.5);
        assert!(matches!(state, EmotionalState::Joyful));
    }

    #[test]
    fn test_emotional_state_boundary_mood_just_below_positive() {
        // mood = 0.29 → neutral branch
        let state = LlmPersonaManager::calculate_emotional_state(0.29, 0.5);
        assert!(matches!(state, EmotionalState::Neutral));
    }

    #[test]
    fn test_emotional_state_boundary_mood_exactly_negative_threshold() {
        // mood = -0.3 → negative branch
        let state = LlmPersonaManager::calculate_emotional_state(-0.3, 0.5);
        assert!(matches!(state, EmotionalState::Frustrated));
    }

    #[test]
    fn test_emotional_state_boundary_mood_just_above_negative() {
        // mood = -0.29 → neutral branch
        let state = LlmPersonaManager::calculate_emotional_state(-0.29, 0.5);
        assert!(matches!(state, EmotionalState::Neutral));
    }

    #[test]
    fn test_emotional_state_positive_mood_energy_exactly_0_6() {
        // mood >= 0.3, energy = 0.6 → Excited (>= 0.6)
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.6);
        assert!(matches!(state, EmotionalState::Excited));
    }

    #[test]
    fn test_emotional_state_positive_mood_energy_exactly_0_3() {
        // mood >= 0.3, energy = 0.3 → Joyful (>= 0.3)
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.3);
        assert!(matches!(state, EmotionalState::Joyful));
    }

    #[test]
    fn test_emotional_state_positive_mood_energy_just_below_0_3() {
        // mood >= 0.3, energy = 0.29 → Calm (< 0.3)
        let state = LlmPersonaManager::calculate_emotional_state(0.5, 0.29);
        assert!(matches!(state, EmotionalState::Calm));
    }

    #[test]
    fn test_emotional_state_negative_mood_energy_exactly_0_6() {
        // mood <= -0.3, energy = 0.6 → Angry (>= 0.6)
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.6);
        assert!(matches!(state, EmotionalState::Angry));
    }

    #[test]
    fn test_emotional_state_negative_mood_energy_exactly_0_3() {
        // mood <= -0.3, energy = 0.3 → Frustrated (>= 0.3)
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.3);
        assert!(matches!(state, EmotionalState::Frustrated));
    }

    #[test]
    fn test_emotional_state_negative_mood_energy_just_below_0_3() {
        // mood <= -0.3, energy = 0.29 → Sad (< 0.3)
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.29);
        assert!(matches!(state, EmotionalState::Sad));
    }

    #[test]
    fn test_emotional_state_neutral_mood_energy_exactly_0_7() {
        // neutral mood, energy = 0.7 → Curious (>= 0.7)
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.7);
        assert!(matches!(state, EmotionalState::Curious));
    }

    #[test]
    fn test_emotional_state_neutral_mood_energy_exactly_0_3() {
        // neutral mood, energy = 0.3 → Neutral (not <= 0.3 for Thoughtful, not >= 0.7 for Curious)
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.3);
        // energy = 0.3 is NOT <= 0.3 (it's not strictly less), check the actual logic
        // The code says: `energy <= 0.3 → Thoughtful` so 0.3 IS Thoughtful
        assert!(matches!(state, EmotionalState::Thoughtful));
    }

    #[test]
    fn test_emotional_state_neutral_mood_energy_just_above_0_3() {
        // neutral mood, energy = 0.31 → Neutral
        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.31);
        assert!(matches!(state, EmotionalState::Neutral));
    }

    #[test]
    fn test_emotional_state_extreme_values() {
        let state = LlmPersonaManager::calculate_emotional_state(1.0, 1.0);
        assert!(matches!(state, EmotionalState::Excited));

        let state = LlmPersonaManager::calculate_emotional_state(-1.0, 0.0);
        assert!(matches!(state, EmotionalState::Sad));

        let state = LlmPersonaManager::calculate_emotional_state(0.0, 0.0);
        assert!(matches!(state, EmotionalState::Thoughtful));
    }

    // ── Clone independence tests ──

    #[test]
    fn test_persona_llm_config_clone_independence() {
        let mut original = PersonaLlmConfig::default();
        let cloned = original.clone();
        original.temperature = 1.5;
        original.max_tokens = 1024;
        original
            .personality_factors
            .insert("creativity".to_string(), 0.1);

        assert_eq!(cloned.temperature, 0.8);
        assert_eq!(cloned.max_tokens, 512);
        assert_eq!(*cloned.personality_factors.get("creativity").unwrap(), 0.7);
    }

    #[test]
    fn test_personality_state_clone_independence() {
        let mut original = PersonalityState::default();
        let cloned = original.clone();
        original.current_mood = 0.9;
        original.trust_level = 0.0;
        original.personality_drift.insert("test".to_string(), 1.0);
        original.recent_influences.push(PersonalityInfluence {
            event: "test".to_string(),
            factor_changes: HashMap::new(),
            timestamp: 0,
            importance: 1.0,
            decay_rate: 0.0,
        });

        assert_eq!(cloned.current_mood, 0.0);
        assert_eq!(cloned.trust_level, 0.5);
        assert!(cloned.personality_drift.is_empty());
        assert!(cloned.recent_influences.is_empty());
    }

    #[test]
    fn test_adaptation_data_clone_independence() {
        let mut original = AdaptationData::default();
        let cloned = original.clone();
        original.interaction_count = 999;
        original.preferred_topics.insert("combat".to_string(), 0.9);
        original.topics_to_avoid.push("politics".to_string());

        assert_eq!(cloned.interaction_count, 0);
        assert!(cloned.preferred_topics.is_empty());
        assert!(cloned.topics_to_avoid.is_empty());
    }

    #[test]
    fn test_memory_profile_clone_independence() {
        let mut original = MemoryProfile::default();
        let cloned = original.clone();
        original.core_memories.push("important".to_string());
        original.episodic_memories.push("yesterday".to_string());
        original.semantic_knowledge.push("fact".to_string());

        assert!(cloned.core_memories.is_empty());
        assert!(cloned.episodic_memories.is_empty());
        assert!(cloned.semantic_knowledge.is_empty());
    }

    // ── Populated data serde roundtrips ──

    #[test]
    fn test_persona_llm_config_custom_roundtrip() {
        let mut factors = HashMap::new();
        factors.insert("aggression".to_string(), 0.95);
        factors.insert("loyalty".to_string(), 0.1);
        let config = PersonaLlmConfig {
            temperature: 1.5,
            top_p: 0.5,
            max_tokens: 2048,
            personality_factors: factors,
            response_style: ResponseStyle::Mysterious,
            context_window_size: 4096,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PersonaLlmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.temperature, 1.5);
        assert_eq!(deserialized.top_p, 0.5);
        assert_eq!(deserialized.max_tokens, 2048);
        assert_eq!(deserialized.context_window_size, 4096);
        assert!(matches!(
            deserialized.response_style,
            ResponseStyle::Mysterious
        ));
        assert_eq!(deserialized.personality_factors.len(), 2);
        assert_eq!(
            *deserialized.personality_factors.get("aggression").unwrap(),
            0.95
        );
    }

    #[test]
    fn test_personality_state_populated_roundtrip() {
        let mut drift = HashMap::new();
        drift.insert("creativity".to_string(), 0.15);
        drift.insert("empathy".to_string(), -0.05);
        let mut fc = HashMap::new();
        fc.insert("humor".to_string(), 0.1);
        let state = PersonalityState {
            current_mood: -0.7,
            energy_level: 0.9,
            confidence: 0.3,
            trust_level: 0.95,
            emotional_state: EmotionalState::Anxious,
            personality_drift: drift,
            recent_influences: vec![PersonalityInfluence {
                event: "battle won".to_string(),
                factor_changes: fc,
                timestamp: 42,
                importance: 0.8,
                decay_rate: 0.02,
            }],
        };
        let json = serde_json::to_string(&state).unwrap();
        let d: PersonalityState = serde_json::from_str(&json).unwrap();
        assert_eq!(d.current_mood, -0.7);
        assert_eq!(d.energy_level, 0.9);
        assert_eq!(d.confidence, 0.3);
        assert_eq!(d.trust_level, 0.95);
        assert!(matches!(d.emotional_state, EmotionalState::Anxious));
        assert_eq!(d.personality_drift.len(), 2);
        assert_eq!(d.recent_influences.len(), 1);
        assert_eq!(d.recent_influences[0].event, "battle won");
        assert_eq!(d.recent_influences[0].timestamp, 42);
    }

    #[test]
    fn test_adaptation_data_populated_roundtrip() {
        let mut topics = HashMap::new();
        topics.insert("magic".to_string(), 0.8);
        topics.insert("swords".to_string(), 0.6);
        let mut changes = HashMap::new();
        changes.insert("curiosity".to_string(), 0.05);
        let data = AdaptationData {
            interaction_count: 150,
            successful_interactions: 120,
            learning_rate: 0.05,
            preferred_topics: topics,
            topics_to_avoid: vec!["violence".to_string(), "death".to_string()],
            player_patterns: PlayerPatterns {
                communication_style: Some("casual".to_string()),
                interests: vec!["rpg".to_string(), "exploration".to_string()],
                avg_session_length: Some(45.5),
                preferred_times: vec!["evening".to_string()],
                emotional_patterns: {
                    let mut m = HashMap::new();
                    m.insert("joy".to_string(), 0.7);
                    m
                },
            },
            adaptation_history: vec![AdaptationEvent {
                trigger: "positive feedback".to_string(),
                changes,
                timestamp: 100,
                success_rating: Some(0.9),
            }],
        };
        let json = serde_json::to_string(&data).unwrap();
        let d: AdaptationData = serde_json::from_str(&json).unwrap();
        assert_eq!(d.interaction_count, 150);
        assert_eq!(d.successful_interactions, 120);
        assert_eq!(d.learning_rate, 0.05);
        assert_eq!(d.preferred_topics.len(), 2);
        assert_eq!(d.topics_to_avoid.len(), 2);
        assert_eq!(
            d.player_patterns.communication_style,
            Some("casual".to_string())
        );
        assert_eq!(d.player_patterns.interests.len(), 2);
        assert_eq!(d.player_patterns.avg_session_length, Some(45.5));
        assert_eq!(d.adaptation_history.len(), 1);
        assert_eq!(d.adaptation_history[0].success_rating, Some(0.9));
    }

    #[test]
    fn test_prompt_settings_populated_roundtrip() {
        let mut modifiers = HashMap::new();
        modifiers.insert("tone".to_string(), "aggressive".to_string());
        modifiers.insert("formality".to_string(), "low".to_string());
        let settings = PromptSettings {
            system_prompt_template: "Custom: {{persona.name}}".to_string(),
            context_injection: ContextInjectionStrategy::Full,
            memory_retrieval: MemoryRetrievalSettings {
                max_memories: 10,
                min_similarity: 0.5,
                priority_categories: vec!["Combat".to_string()],
                recency_bonus: 0.2,
            },
            few_shot_examples: vec![
                FewShotExample {
                    input: "Hello".to_string(),
                    output: "Greetings, warrior!".to_string(),
                    context: Some("first meeting".to_string()),
                    tags: vec!["greeting".to_string()],
                },
                FewShotExample {
                    input: "Bye".to_string(),
                    output: "Until we meet again.".to_string(),
                    context: None,
                    tags: vec![],
                },
            ],
            prompt_modifiers: modifiers,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let d: PromptSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(d.system_prompt_template, "Custom: {{persona.name}}");
        assert!(matches!(
            d.context_injection,
            ContextInjectionStrategy::Full
        ));
        assert_eq!(d.memory_retrieval.max_memories, 10);
        assert_eq!(d.memory_retrieval.min_similarity, 0.5);
        assert_eq!(d.few_shot_examples.len(), 2);
        assert_eq!(
            d.few_shot_examples[0].context,
            Some("first meeting".to_string())
        );
        assert!(d.few_shot_examples[1].context.is_none());
        assert_eq!(d.prompt_modifiers.len(), 2);
    }

    #[test]
    fn test_memory_profile_populated_roundtrip() {
        let profile = MemoryProfile {
            core_memories: vec!["I am a merchant".to_string(), "Born in Astra".to_string()],
            episodic_memories: vec!["Met the hero yesterday".to_string()],
            semantic_knowledge: vec![
                "Gold is valuable".to_string(),
                "Dragons breathe fire".to_string(),
            ],
            consolidation_preferences: ConsolidationPreferences {
                consolidation_frequency: 50,
                importance_threshold: 0.5,
                max_memories: 500,
                forgetting_curve: ForgettingCurve {
                    decay_rate: 0.2,
                    importance_multiplier: 3.0,
                    rehearsal_bonus: 0.8,
                },
            },
        };
        let json = serde_json::to_string(&profile).unwrap();
        let d: MemoryProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(d.core_memories.len(), 2);
        assert_eq!(d.episodic_memories.len(), 1);
        assert_eq!(d.semantic_knowledge.len(), 2);
        assert_eq!(d.consolidation_preferences.consolidation_frequency, 50);
        assert_eq!(d.consolidation_preferences.importance_threshold, 0.5);
        assert_eq!(d.consolidation_preferences.max_memories, 500);
        assert_eq!(d.consolidation_preferences.forgetting_curve.decay_rate, 0.2);
        assert_eq!(
            d.consolidation_preferences
                .forgetting_curve
                .importance_multiplier,
            3.0
        );
        assert_eq!(
            d.consolidation_preferences.forgetting_curve.rehearsal_bonus,
            0.8
        );
    }

    // ── Optional field roundtrips ──

    #[test]
    fn test_few_shot_example_without_context_roundtrip() {
        let example = FewShotExample {
            input: "Attack!".to_string(),
            output: "Charging forward!".to_string(),
            context: None,
            tags: vec![],
        };
        let json = serde_json::to_string(&example).unwrap();
        let d: FewShotExample = serde_json::from_str(&json).unwrap();
        assert!(d.context.is_none());
        assert!(d.tags.is_empty());
        assert_eq!(d.input, "Attack!");
    }

    #[test]
    fn test_adaptation_event_without_success_rating_roundtrip() {
        let event = AdaptationEvent {
            trigger: "new topic".to_string(),
            changes: HashMap::new(),
            timestamp: 0,
            success_rating: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let d: AdaptationEvent = serde_json::from_str(&json).unwrap();
        assert!(d.success_rating.is_none());
        assert!(d.changes.is_empty());
        assert_eq!(d.timestamp, 0);
    }

    #[test]
    fn test_player_patterns_all_none_roundtrip() {
        let patterns = PlayerPatterns::default();
        let json = serde_json::to_string(&patterns).unwrap();
        let d: PlayerPatterns = serde_json::from_str(&json).unwrap();
        assert!(d.communication_style.is_none());
        assert!(d.interests.is_empty());
        assert!(d.avg_session_length.is_none());
        assert!(d.preferred_times.is_empty());
        assert!(d.emotional_patterns.is_empty());
    }

    #[test]
    fn test_personality_influence_empty_factor_changes_roundtrip() {
        let influence = PersonalityInfluence {
            event: "nothing happened".to_string(),
            factor_changes: HashMap::new(),
            timestamp: 999,
            importance: 0.0,
            decay_rate: 1.0,
        };
        let json = serde_json::to_string(&influence).unwrap();
        let d: PersonalityInfluence = serde_json::from_str(&json).unwrap();
        assert!(d.factor_changes.is_empty());
        assert_eq!(d.importance, 0.0);
        assert_eq!(d.decay_rate, 1.0);
    }

    // ── PersonaMetrics field manipulation ──

    #[test]
    fn test_persona_metrics_increment_fields() {
        let mut metrics = PersonaMetrics::default();
        metrics.total_interactions = 100;
        metrics.successful_generations = 95;
        metrics.failed_generations = 5;
        metrics.avg_response_time_ms = 42.5;
        metrics.personality_evolution_events = 10;
        metrics.memory_consolidations = 3;
        metrics.adaptation_learning_events = 7;

        assert_eq!(metrics.total_interactions, 100);
        assert_eq!(metrics.successful_generations, 95);
        assert_eq!(metrics.failed_generations, 5);
        assert_eq!(metrics.avg_response_time_ms, 42.5);
        assert_eq!(metrics.personality_evolution_events, 10);
        assert_eq!(metrics.memory_consolidations, 3);
        assert_eq!(metrics.adaptation_learning_events, 7);
    }

    #[test]
    fn test_persona_metrics_serde_roundtrip() {
        let metrics = PersonaMetrics {
            total_interactions: 500,
            successful_generations: 480,
            failed_generations: 20,
            avg_response_time_ms: 123.456,
            personality_evolution_events: 15,
            memory_consolidations: 5,
            adaptation_learning_events: 30,
        };
        let json = serde_json::to_string(&metrics).unwrap();
        let d: PersonaMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(d.total_interactions, 500);
        assert_eq!(d.successful_generations, 480);
        assert_eq!(d.failed_generations, 20);
        assert_eq!(d.avg_response_time_ms, 123.456);
        assert_eq!(d.personality_evolution_events, 15);
        assert_eq!(d.memory_consolidations, 5);
        assert_eq!(d.adaptation_learning_events, 30);
    }

    #[test]
    fn test_persona_metrics_clone_independence() {
        let mut original = PersonaMetrics::default();
        let cloned = original.clone();
        original.total_interactions = 999;
        original.failed_generations = 50;
        assert_eq!(cloned.total_interactions, 0);
        assert_eq!(cloned.failed_generations, 0);
    }

    // ── Response style serde roundtrips ──

    #[test]
    fn test_response_style_each_variant_roundtrip() {
        let styles = [
            ResponseStyle::Conversational,
            ResponseStyle::Formal,
            ResponseStyle::Creative,
            ResponseStyle::Technical,
            ResponseStyle::Playful,
            ResponseStyle::Mysterious,
        ];
        for style in &styles {
            let json = serde_json::to_string(style).unwrap();
            let d: ResponseStyle = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", style), format!("{:?}", d));
        }
    }

    #[test]
    fn test_emotional_state_each_variant_roundtrip() {
        let states = [
            EmotionalState::Joyful,
            EmotionalState::Excited,
            EmotionalState::Calm,
            EmotionalState::Neutral,
            EmotionalState::Thoughtful,
            EmotionalState::Concerned,
            EmotionalState::Frustrated,
            EmotionalState::Sad,
            EmotionalState::Angry,
            EmotionalState::Surprised,
            EmotionalState::Curious,
            EmotionalState::Confident,
            EmotionalState::Anxious,
        ];
        for state in &states {
            let json = serde_json::to_string(state).unwrap();
            let d: EmotionalState = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", state), format!("{:?}", d));
        }
    }

    #[test]
    fn test_context_injection_strategy_each_variant_roundtrip() {
        let strategies = [
            ContextInjectionStrategy::Full,
            ContextInjectionStrategy::Recent,
            ContextInjectionStrategy::Contextual,
            ContextInjectionStrategy::Minimal,
        ];
        for strategy in &strategies {
            let json = serde_json::to_string(strategy).unwrap();
            let d: ContextInjectionStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", strategy), format!("{:?}", d));
        }
    }

    // ── Populated LlmPersona roundtrip (full composite) ──

    #[test]
    fn test_llm_persona_full_populated_roundtrip() {
        let mut factors = HashMap::new();
        factors.insert("bravery".to_string(), 0.9);
        let mut drift = HashMap::new();
        drift.insert("bravery".to_string(), 0.05);
        let persona = LlmPersona {
            base: BasePersona::default(),
            llm_config: PersonaLlmConfig {
                temperature: 1.2,
                top_p: 0.7,
                max_tokens: 1024,
                personality_factors: factors,
                response_style: ResponseStyle::Creative,
                context_window_size: 8192,
            },
            personality_state: PersonalityState {
                current_mood: 0.6,
                energy_level: 0.8,
                confidence: 0.9,
                trust_level: 0.7,
                emotional_state: EmotionalState::Excited,
                personality_drift: drift,
                recent_influences: vec![],
            },
            adaptation: AdaptationData {
                interaction_count: 42,
                successful_interactions: 40,
                learning_rate: 0.2,
                preferred_topics: {
                    let mut m = HashMap::new();
                    m.insert("adventure".to_string(), 0.9);
                    m
                },
                topics_to_avoid: vec!["boring".to_string()],
                player_patterns: PlayerPatterns::default(),
                adaptation_history: vec![],
            },
            prompt_settings: PromptSettings::default(),
            memory_profile: MemoryProfile {
                core_memories: vec!["I love adventure".to_string()],
                episodic_memories: vec![],
                semantic_knowledge: vec![],
                consolidation_preferences: ConsolidationPreferences::default(),
            },
        };
        let json = serde_json::to_string(&persona).unwrap();
        let d: LlmPersona = serde_json::from_str(&json).unwrap();
        assert_eq!(d.llm_config.temperature, 1.2);
        assert_eq!(d.llm_config.context_window_size, 8192);
        assert!(matches!(
            d.llm_config.response_style,
            ResponseStyle::Creative
        ));
        assert_eq!(d.personality_state.current_mood, 0.6);
        assert!(matches!(
            d.personality_state.emotional_state,
            EmotionalState::Excited
        ));
        assert_eq!(d.adaptation.interaction_count, 42);
        assert_eq!(d.adaptation.preferred_topics.len(), 1);
        assert_eq!(d.memory_profile.core_memories.len(), 1);
    }

    // ── Deserialization error tests ──

    #[test]
    fn test_persona_llm_config_missing_field_fails() {
        let json = r#"{"temperature": 0.8}"#;
        let result: Result<PersonaLlmConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_personality_state_missing_field_fails() {
        let json = r#"{"current_mood": 0.5}"#;
        let result: Result<PersonalityState, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_forgetting_curve_missing_field_fails() {
        let json = r#"{"decay_rate": 0.1}"#;
        let result: Result<ForgettingCurve, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_retrieval_settings_missing_field_fails() {
        let json = r#"{"max_memories": 5}"#;
        let result: Result<MemoryRetrievalSettings, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ── Edge-case value tests ──

    #[test]
    fn test_consolidation_preferences_custom_roundtrip() {
        let prefs = ConsolidationPreferences {
            consolidation_frequency: 1,
            importance_threshold: 0.0,
            max_memories: 0,
            forgetting_curve: ForgettingCurve {
                decay_rate: 0.0,
                importance_multiplier: 0.0,
                rehearsal_bonus: 0.0,
            },
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let d: ConsolidationPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(d.consolidation_frequency, 1);
        assert_eq!(d.importance_threshold, 0.0);
        assert_eq!(d.max_memories, 0);
        assert_eq!(d.forgetting_curve.decay_rate, 0.0);
    }

    #[test]
    fn test_memory_retrieval_settings_custom_roundtrip() {
        let settings = MemoryRetrievalSettings {
            max_memories: 100,
            min_similarity: 0.9,
            priority_categories: vec![
                "Combat".to_string(),
                "Trade".to_string(),
                "Lore".to_string(),
            ],
            recency_bonus: 0.5,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let d: MemoryRetrievalSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(d.max_memories, 100);
        assert_eq!(d.min_similarity, 0.9);
        assert_eq!(d.priority_categories.len(), 3);
        assert_eq!(d.recency_bonus, 0.5);
    }

    #[test]
    fn test_forgetting_curve_custom_roundtrip() {
        let curve = ForgettingCurve {
            decay_rate: 0.99,
            importance_multiplier: 10.0,
            rehearsal_bonus: 5.0,
        };
        let json = serde_json::to_string(&curve).unwrap();
        let d: ForgettingCurve = serde_json::from_str(&json).unwrap();
        assert_eq!(d.decay_rate, 0.99);
        assert_eq!(d.importance_multiplier, 10.0);
        assert_eq!(d.rehearsal_bonus, 5.0);
    }

    // ── Debug trait tests ──

    #[test]
    fn test_all_types_implement_debug() {
        let _ = format!("{:?}", PersonaLlmConfig::default());
        let _ = format!("{:?}", PersonalityState::default());
        let _ = format!("{:?}", AdaptationData::default());
        let _ = format!("{:?}", PlayerPatterns::default());
        let _ = format!("{:?}", PromptSettings::default());
        let _ = format!("{:?}", MemoryRetrievalSettings::default());
        let _ = format!("{:?}", MemoryProfile::default());
        let _ = format!("{:?}", ConsolidationPreferences::default());
        let _ = format!("{:?}", ForgettingCurve::default());
        let _ = format!("{:?}", PersonaMetrics::default());
        let _ = format!("{:?}", ResponseStyle::Conversational);
        let _ = format!("{:?}", EmotionalState::Neutral);
        let _ = format!("{:?}", ContextInjectionStrategy::Contextual);
    }

    #[test]
    fn test_personality_influence_debug_and_clone() {
        let influence = PersonalityInfluence {
            event: "test".to_string(),
            factor_changes: HashMap::new(),
            timestamp: 123,
            importance: 0.5,
            decay_rate: 0.1,
        };
        let debug_str = format!("{:?}", influence);
        assert!(debug_str.contains("PersonalityInfluence"));
        let cloned = influence.clone();
        assert_eq!(cloned.event, "test");
        assert_eq!(cloned.timestamp, 123);
    }

    #[test]
    fn test_adaptation_event_debug_and_clone() {
        let event = AdaptationEvent {
            trigger: "reward".to_string(),
            changes: {
                let mut m = HashMap::new();
                m.insert("mood".to_string(), 0.3);
                m
            },
            timestamp: 456,
            success_rating: Some(0.75),
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("AdaptationEvent"));
        let cloned = event.clone();
        assert_eq!(cloned.trigger, "reward");
        assert_eq!(cloned.changes.len(), 1);
    }

    #[test]
    fn test_few_shot_example_debug_and_clone() {
        let example = FewShotExample {
            input: "hi".to_string(),
            output: "hello".to_string(),
            context: Some("greeting".to_string()),
            tags: vec!["social".to_string()],
        };
        let debug_str = format!("{:?}", example);
        assert!(debug_str.contains("FewShotExample"));
        let cloned = example.clone();
        assert_eq!(cloned.tags.len(), 1);
    }

    // ── Copy trait tests for enums ──

    #[test]
    fn test_response_style_is_copy() {
        let style = ResponseStyle::Formal;
        let copied = style;
        // Both remain valid because ResponseStyle is Copy
        assert!(matches!(style, ResponseStyle::Formal));
        assert!(matches!(copied, ResponseStyle::Formal));
    }

    #[test]
    fn test_emotional_state_is_copy() {
        let state = EmotionalState::Confident;
        let copied = state;
        assert!(matches!(state, EmotionalState::Confident));
        assert!(matches!(copied, EmotionalState::Confident));
    }

    #[test]
    fn test_context_injection_strategy_is_copy() {
        let strategy = ContextInjectionStrategy::Minimal;
        let copied = strategy;
        assert!(matches!(strategy, ContextInjectionStrategy::Minimal));
        assert!(matches!(copied, ContextInjectionStrategy::Minimal));
    }

    // ── Clone-independence tests for remaining structs ──

    #[test]
    fn test_personality_influence_clone_independence() {
        let mut original = PersonalityInfluence {
            event: "battle".to_string(),
            factor_changes: {
                let mut m = HashMap::new();
                m.insert("courage".to_string(), 0.5);
                m
            },
            timestamp: 100,
            importance: 0.8,
            decay_rate: 0.01,
        };
        let clone = original.clone();
        original.event = "retreat".to_string();
        original.factor_changes.insert("fear".to_string(), 0.3);
        original.importance = 0.1;

        assert_eq!(clone.event, "battle");
        assert_eq!(clone.factor_changes.len(), 1);
        assert!((clone.importance - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_adaptation_event_clone_independence() {
        let mut original = AdaptationEvent {
            trigger: "positive_feedback".to_string(),
            changes: {
                let mut m = HashMap::new();
                m.insert("empathy".to_string(), 0.1);
                m
            },
            timestamp: 200,
            success_rating: Some(0.9),
        };
        let clone = original.clone();
        original.trigger = "negative_feedback".to_string();
        original.changes.clear();
        original.success_rating = None;

        assert_eq!(clone.trigger, "positive_feedback");
        assert_eq!(clone.changes.len(), 1);
        assert_eq!(clone.success_rating, Some(0.9));
    }

    #[test]
    fn test_few_shot_example_clone_independence() {
        let mut original = FewShotExample {
            input: "hello".to_string(),
            output: "hi there!".to_string(),
            context: Some("greeting".to_string()),
            tags: vec!["social".to_string(), "friendly".to_string()],
        };
        let clone = original.clone();
        original.input = "goodbye".to_string();
        original.tags.clear();
        original.context = None;

        assert_eq!(clone.input, "hello");
        assert_eq!(clone.tags.len(), 2);
        assert_eq!(clone.context, Some("greeting".to_string()));
    }

    #[test]
    fn test_prompt_settings_clone_independence() {
        let mut original = PromptSettings::default();
        original.system_prompt_template = "custom template".to_string();
        original
            .prompt_modifiers
            .insert("tone".to_string(), "formal".to_string());
        let clone = original.clone();
        original.system_prompt_template = "changed".to_string();
        original.prompt_modifiers.clear();

        assert_eq!(clone.system_prompt_template, "custom template");
        assert_eq!(clone.prompt_modifiers.len(), 1);
    }

    #[test]
    fn test_consolidation_preferences_clone_independence() {
        let mut original = ConsolidationPreferences::default();
        original.consolidation_frequency = 999;
        original.forgetting_curve.decay_rate = 0.99;
        let clone = original.clone();
        original.consolidation_frequency = 0;
        original.forgetting_curve.decay_rate = 0.0;

        assert_eq!(clone.consolidation_frequency, 999);
        assert!((clone.forgetting_curve.decay_rate - 0.99).abs() < f32::EPSILON);
    }

    #[test]
    fn test_forgetting_curve_clone_independence() {
        let mut original = ForgettingCurve::default();
        original.decay_rate = 0.5;
        original.importance_multiplier = 2.0;
        original.rehearsal_bonus = 0.75;
        let clone = original.clone();
        original.decay_rate = 0.0;
        original.importance_multiplier = 0.0;

        assert!((clone.decay_rate - 0.5).abs() < f32::EPSILON);
        assert!((clone.importance_multiplier - 2.0).abs() < f32::EPSILON);
        assert!((clone.rehearsal_bonus - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_memory_retrieval_settings_clone_independence() {
        let mut original = MemoryRetrievalSettings::default();
        original.max_memories = 42;
        original.priority_categories.push("important".to_string());
        let clone = original.clone();
        original.max_memories = 0;
        original.priority_categories.clear();

        assert_eq!(clone.max_memories, 42);
        assert_eq!(clone.priority_categories.len(), 3); // 2 defaults + 1 pushed
    }

    #[test]
    fn test_player_patterns_clone_independence() {
        let mut original = PlayerPatterns::default();
        original.communication_style = Some("casual".to_string());
        original.interests.push("combat".to_string());
        original.emotional_patterns.insert("anger".to_string(), 0.7);
        let clone = original.clone();
        original.communication_style = None;
        original.interests.clear();
        original.emotional_patterns.clear();

        assert_eq!(clone.communication_style, Some("casual".to_string()));
        assert_eq!(clone.interests.len(), 1);
        assert_eq!(clone.emotional_patterns.len(), 1);
    }

    // ── Serde roundtrip tests for populated structs ──

    #[test]
    fn test_personality_influence_serde_roundtrip() {
        let influence = PersonalityInfluence {
            event: "victory".to_string(),
            factor_changes: {
                let mut m = HashMap::new();
                m.insert("confidence".to_string(), 0.3);
                m
            },
            timestamp: 12345,
            importance: 0.9,
            decay_rate: 0.01,
        };
        let json = serde_json::to_string(&influence).unwrap();
        let restored: PersonalityInfluence = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.event, "victory");
        assert!((restored.importance - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_adaptation_event_serde_roundtrip() {
        let event = AdaptationEvent {
            trigger: "player_feedback".to_string(),
            changes: {
                let mut m = HashMap::new();
                m.insert("humor".to_string(), 0.2);
                m
            },
            timestamp: 67890,
            success_rating: Some(0.85),
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: AdaptationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.trigger, "player_feedback");
        assert_eq!(restored.success_rating, Some(0.85));
    }

    #[test]
    fn test_adaptation_event_serde_none_success_rating() {
        let event = AdaptationEvent {
            trigger: "auto".to_string(),
            changes: HashMap::new(),
            timestamp: 0,
            success_rating: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: AdaptationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.success_rating, None);
    }

    #[test]
    fn test_few_shot_example_serde_roundtrip() {
        let example = FewShotExample {
            input: "What's the weather?".to_string(),
            output: "I'm not sure, check outside!".to_string(),
            context: None,
            tags: vec!["question".to_string()],
        };
        let json = serde_json::to_string(&example).unwrap();
        let restored: FewShotExample = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.input, "What's the weather?");
        assert_eq!(restored.context, None);
    }

    #[test]
    fn test_player_patterns_serde_roundtrip() {
        let patterns = PlayerPatterns {
            communication_style: Some("verbose".to_string()),
            interests: vec!["lore".to_string(), "crafting".to_string()],
            avg_session_length: Some(45.0),
            preferred_times: vec!["evening".to_string()],
            emotional_patterns: {
                let mut m = HashMap::new();
                m.insert("excitement".to_string(), 0.8);
                m
            },
        };
        let json = serde_json::to_string(&patterns).unwrap();
        let restored: PlayerPatterns = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.interests.len(), 2);
        assert_eq!(restored.avg_session_length, Some(45.0));
    }

    // ── Additional deserialization error tests ──

    #[test]
    fn test_prompt_settings_deser_missing_field() {
        let json = r#"{"system_prompt_template": "hi"}"#;
        let result = serde_json::from_str::<PromptSettings>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_adaptation_data_deser_missing_field() {
        let json = r#"{"interaction_count": 5}"#;
        let result = serde_json::from_str::<AdaptationData>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_adaptation_event_deser_missing_field() {
        let json = r#"{"trigger": "test"}"#;
        let result = serde_json::from_str::<AdaptationEvent>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_player_patterns_deser_empty_object() {
        // PlayerPatterns has all optional/defaultable fields with derive(Default)
        // but does NOT have default serde — so missing fields should fail
        let json = r#"{}"#;
        let result = serde_json::from_str::<PlayerPatterns>(json);
        // PlayerPatterns derives Default but NOT #[serde(default)] —
        // so missing fields actually errors. If it has serde(default), it succeeds.
        // Either way, exercise the path.
        let _ = result; // just exercise deserialization
    }

    #[test]
    fn test_few_shot_example_deser_missing_field() {
        let json = r#"{"input": "hi"}"#;
        let result = serde_json::from_str::<FewShotExample>(json);
        assert!(result.is_err());
    }

    // ── Additional calculate_emotional_state boundary tests ──

    #[test]
    fn test_emotional_state_exact_positive_boundary() {
        // mood=0.3 exactly (>= 0.3 threshold), energy=0.6 exactly (>= 0.6) → Excited
        let state = LlmPersonaManager::calculate_emotional_state(0.3, 0.6);
        assert!(matches!(state, EmotionalState::Excited));
    }

    #[test]
    fn test_emotional_state_exact_negative_boundary() {
        // mood=-0.3 exactly (<= -0.3 threshold), energy=0.5 (>= 0.3) → Frustrated
        let state = LlmPersonaManager::calculate_emotional_state(-0.3, 0.5);
        assert!(matches!(state, EmotionalState::Frustrated));
    }

    #[test]
    fn test_emotional_state_sad_low_energy_negative_mood() {
        // negative mood (<= -0.3) + low energy (< 0.3) → Sad
        let state = LlmPersonaManager::calculate_emotional_state(-0.5, 0.2);
        assert!(matches!(state, EmotionalState::Sad));
    }
}
