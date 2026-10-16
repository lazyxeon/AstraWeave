/*!
# LLM-Enhanced Dialogue System

Advanced dialogue system with LLM integration for dynamic conversation generation,
emotional intelligence, context awareness, and adaptive branching.
*/

use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

// Base dialogue system
use crate::DialogueGraph;

// LLM Integration
use astraweave_context::{ContextConfig, ConversationHistory, Role};
use astraweave_llm::LlmClient;
use astraweave_persona::LlmPersona;
use astraweave_prompts::context::PromptContext as TemplateContext;
use astraweave_prompts::engine::TemplateEngine;
use astraweave_prompts::template::PromptTemplate;
use astraweave_rag::{MemoryQuery, RagPipeline, RetrievedMemory};

/// Enhanced dialogue system with LLM capabilities
pub struct LlmDialogueSystem {
    /// LLM client for generation
    llm_client: Arc<dyn LlmClient>,

    /// RAG pipeline for context retrieval
    rag_pipeline: Arc<RwLock<RagPipeline>>,

    /// Template engine for prompt generation
    template_engine: Arc<RwLock<TemplateEngine>>,

    /// Active conversations. Each conversation has its own mutex so one
    /// `generate_response` updates history, metadata, and emotion together.
    conversations: Arc<RwLock<HashMap<String, Arc<Mutex<ActiveConversation>>>>>,

    /// Dialogue configuration
    config: DialogueConfig,

    /// Performance metrics
    metrics: Arc<RwLock<DialogueMetrics>>,
}

/// Configuration for LLM dialogue system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueConfig {
    /// Maximum response length in tokens
    pub max_response_tokens: usize,

    /// Temperature for dialogue generation
    pub temperature: f32,

    /// Top-p for dialogue generation
    pub top_p: f32,

    /// Number of dialogue options to generate
    pub num_dialogue_options: usize,

    /// Emotional analysis settings
    pub emotion_analysis: EmotionAnalysisConfig,

    /// Context settings
    pub context_config: DialogueContextConfig,

    /// Branching settings
    pub branching_config: BranchingConfig,

    /// Quality control settings
    pub quality_control: QualityControlConfig,
}

impl Default for DialogueConfig {
    fn default() -> Self {
        Self {
            max_response_tokens: 256,
            temperature: 0.8,
            top_p: 0.9,
            num_dialogue_options: 3,
            emotion_analysis: EmotionAnalysisConfig::default(),
            context_config: DialogueContextConfig::default(),
            branching_config: BranchingConfig::default(),
            quality_control: QualityControlConfig::default(),
        }
    }
}

impl DialogueConfig {
    /// Creates a new dialogue config with custom token limit.
    #[must_use]
    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_response_tokens = tokens;
        self
    }

    /// Creates a new dialogue config with custom temperature.
    #[must_use]
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp.clamp(0.0, 2.0);
        self
    }

    /// Creates a new dialogue config with custom top_p.
    #[must_use]
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p.clamp(0.0, 1.0);
        self
    }

    /// Returns true if emotion analysis is enabled.
    #[must_use]
    pub fn has_emotion_analysis(&self) -> bool {
        self.emotion_analysis.enable_emotion_detection
            || self.emotion_analysis.enable_sentiment_analysis
    }

    /// Returns true if quality control is enabled.
    #[must_use]
    pub fn has_quality_control(&self) -> bool {
        self.quality_control.enable_validation
    }

    /// Returns true if dynamic branching is enabled.
    #[must_use]
    pub fn has_dynamic_branching(&self) -> bool {
        self.branching_config.enable_dynamic_branching
    }

    /// Returns a brief summary of this config.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "DialogueConfig: {} tokens, temp={:.1}, {} options",
            self.max_response_tokens, self.temperature, self.num_dialogue_options
        )
    }
}

impl std::fmt::Display for DialogueConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Configuration for emotional analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionAnalysisConfig {
    /// Enable sentiment analysis
    pub enable_sentiment_analysis: bool,

    /// Enable emotion detection
    pub enable_emotion_detection: bool,

    /// Enable empathy responses
    pub enable_empathy_responses: bool,

    /// Sentiment influence on responses
    pub sentiment_influence: f32,

    /// Emotional memory retention
    pub emotional_memory_retention: f32,
}

impl Default for EmotionAnalysisConfig {
    fn default() -> Self {
        Self {
            enable_sentiment_analysis: true,
            enable_emotion_detection: true,
            enable_empathy_responses: true,
            sentiment_influence: 0.3,
            emotional_memory_retention: 0.7,
        }
    }
}

impl EmotionAnalysisConfig {
    /// Creates a disabled emotion analysis config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enable_sentiment_analysis: false,
            enable_emotion_detection: false,
            enable_empathy_responses: false,
            sentiment_influence: 0.0,
            emotional_memory_retention: 0.0,
        }
    }

    /// Returns true if any analysis is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enable_sentiment_analysis || self.enable_emotion_detection
    }

    /// Returns true if all analysis is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        !self.is_enabled()
    }

    /// Returns a count of enabled features.
    #[must_use]
    pub fn enabled_feature_count(&self) -> usize {
        let mut count = 0;
        if self.enable_sentiment_analysis {
            count += 1;
        }
        if self.enable_emotion_detection {
            count += 1;
        }
        if self.enable_empathy_responses {
            count += 1;
        }
        count
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.is_disabled() {
            "EmotionAnalysis: disabled".to_string()
        } else {
            format!(
                "EmotionAnalysis: {} features enabled",
                self.enabled_feature_count()
            )
        }
    }
}

impl std::fmt::Display for EmotionAnalysisConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Configuration for dialogue context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueContextConfig {
    /// Maximum conversation history to consider
    pub max_history_turns: usize,

    /// Number of relevant memories to retrieve
    pub max_relevant_memories: usize,

    /// Context window size in tokens
    pub context_window_size: usize,

    /// Enable conversation summarization
    pub enable_summarization: bool,
}

impl Default for DialogueContextConfig {
    fn default() -> Self {
        Self {
            max_history_turns: 10,
            max_relevant_memories: 5,
            context_window_size: 2048,
            enable_summarization: true,
        }
    }
}

impl DialogueContextConfig {
    /// Creates a minimal context config for low memory usage.
    #[must_use]
    pub fn minimal() -> Self {
        Self {
            max_history_turns: 3,
            max_relevant_memories: 2,
            context_window_size: 512,
            enable_summarization: false,
        }
    }

    /// Creates an extended context config for detailed conversations.
    #[must_use]
    pub fn extended() -> Self {
        Self {
            max_history_turns: 20,
            max_relevant_memories: 10,
            context_window_size: 4096,
            enable_summarization: true,
        }
    }

    /// Returns the total context capacity.
    #[must_use]
    pub fn total_context_items(&self) -> usize {
        self.max_history_turns + self.max_relevant_memories
    }

    /// Returns true if summarization is enabled.
    #[must_use]
    pub fn has_summarization(&self) -> bool {
        self.enable_summarization
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "ContextConfig: {} turns, {} memories, {} tokens",
            self.max_history_turns, self.max_relevant_memories, self.context_window_size
        )
    }
}

impl std::fmt::Display for DialogueContextConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Configuration for dialogue branching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchingConfig {
    /// Enable dynamic branching
    pub enable_dynamic_branching: bool,

    /// Branching probability threshold
    pub branching_threshold: f32,

    /// Maximum branch depth
    pub max_branch_depth: usize,

    /// Branch merge strategy
    pub merge_strategy: BranchMergeStrategy,
}

impl Default for BranchingConfig {
    fn default() -> Self {
        Self {
            enable_dynamic_branching: true,
            branching_threshold: 0.7,
            max_branch_depth: 5,
            merge_strategy: BranchMergeStrategy::Contextual,
        }
    }
}

impl BranchingConfig {
    /// Creates a disabled branching config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enable_dynamic_branching: false,
            branching_threshold: 1.0,
            max_branch_depth: 0,
            merge_strategy: BranchMergeStrategy::None,
        }
    }

    /// Returns true if branching is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enable_dynamic_branching
    }

    /// Returns true if branching is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        !self.enable_dynamic_branching
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.is_disabled() {
            "BranchingConfig: disabled".to_string()
        } else {
            format!(
                "BranchingConfig: threshold={:.1}, depth={}, strategy={:?}",
                self.branching_threshold, self.max_branch_depth, self.merge_strategy
            )
        }
    }
}

impl std::fmt::Display for BranchingConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl BranchMergeStrategy {
    /// Returns the name of this strategy.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Contextual => "Contextual",
            Self::Emotional => "Emotional",
            Self::PersonaBased => "Persona-Based",
            Self::None => "None",
        }
    }

    /// Returns true if this is a merging strategy.
    #[must_use]
    pub fn is_merging(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns all available strategies.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Contextual,
            Self::Emotional,
            Self::PersonaBased,
            Self::None,
        ]
    }
}

impl std::fmt::Display for BranchMergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Strategies for merging dialogue branches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BranchMergeStrategy {
    /// Merge based on context similarity
    Contextual,
    /// Merge based on emotional state
    Emotional,
    /// Merge based on persona consistency
    PersonaBased,
    /// No automatic merging
    None,
}

/// Configuration for quality control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityControlConfig {
    /// Enable response validation
    pub enable_validation: bool,

    /// Minimum response quality score
    pub min_quality_score: f32,

    /// Enable profanity filtering
    pub enable_profanity_filter: bool,

    /// Enable repetition detection
    pub enable_repetition_detection: bool,

    /// Maximum retry attempts for poor quality
    pub max_retry_attempts: usize,
}

impl Default for QualityControlConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            min_quality_score: 0.6,
            enable_profanity_filter: true,
            enable_repetition_detection: true,
            max_retry_attempts: 3,
        }
    }
}

impl QualityControlConfig {
    /// Creates a disabled quality control config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enable_validation: false,
            min_quality_score: 0.0,
            enable_profanity_filter: false,
            enable_repetition_detection: false,
            max_retry_attempts: 0,
        }
    }

    /// Creates a strict quality control config.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            enable_validation: true,
            min_quality_score: 0.8,
            enable_profanity_filter: true,
            enable_repetition_detection: true,
            max_retry_attempts: 5,
        }
    }

    /// Returns true if validation is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enable_validation
    }

    /// Returns true if all controls are disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        !self.enable_validation
    }

    /// Returns the count of enabled filters.
    #[must_use]
    pub fn enabled_filter_count(&self) -> usize {
        let mut count = 0;
        if self.enable_validation {
            count += 1;
        }
        if self.enable_profanity_filter {
            count += 1;
        }
        if self.enable_repetition_detection {
            count += 1;
        }
        count
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.is_disabled() {
            "QualityControl: disabled".to_string()
        } else {
            format!(
                "QualityControl: min_score={:.1}, {} filters, {} retries",
                self.min_quality_score,
                self.enabled_filter_count(),
                self.max_retry_attempts
            )
        }
    }
}

impl std::fmt::Display for QualityControlConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Model text is a proposal. The sim validates and executes; this helper does not.
///
/// `SimRng` is seeded with `(world_seed, tick, SIM_SYSTEM_LLM_DIALOGUE + entity)` and is not drawn.
/// A missing recording becomes `scripted:` plus the prompt. No live model and no wall clock.
#[must_use]
pub fn recorded_line(
    world_seed: u64,
    tick: u64,
    entity: u64,
    recorded: Option<&str>,
    prompt: &str,
) -> String {
    let stream = astraweave_core::SimRng::new(
        world_seed,
        tick,
        astraweave_core::SIM_SYSTEM_LLM_DIALOGUE.wrapping_add(entity),
    );
    let _seed = (stream.world_seed(), stream.tick(), stream.system_id());
    match recorded {
        Some(line) => line.to_string(),
        None => format!("scripted:{prompt}"),
    }
}

/// An active conversation with context and state.
///
/// The map holds one of these behind a mutex. `history` stays in an `Arc` so a
/// snapshot from `get_conversation` can share the log without copying the client.
#[derive(Clone)]
pub struct ActiveConversation {
    /// Conversation ID
    pub id: String,

    /// Participant personas
    pub participants: Vec<String>,

    /// Conversation history
    pub history: Arc<ConversationHistory>,

    /// Current emotional state
    pub emotional_state: EmotionalState,

    /// Conversation metadata
    pub metadata: ConversationMetadata,

    /// Current dialogue graph (if using structured dialogue)
    pub current_graph: Option<DialogueGraph>,

    /// Current node in the graph
    pub current_node_id: Option<String>,

    /// Dynamic branching state
    pub branching_state: BranchingState,
}

impl std::fmt::Debug for ActiveConversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActiveConversation")
            .field("id", &self.id)
            .field("participants", &self.participants)
            .field("current_node_id", &self.current_node_id)
            .finish_non_exhaustive()
    }
}

/// Current emotional state of a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    /// Overall sentiment (-1.0 to 1.0)
    pub sentiment: f32,

    /// Detected emotions with confidence scores
    pub emotions: HashMap<String, f32>,

    /// Emotional intensity (0.0 to 1.0)
    pub intensity: f32,

    /// Emotional valence (-1.0 to 1.0, negative to positive)
    pub valence: f32,

    /// Emotional arousal (0.0 to 1.0, calm to excited)
    pub arousal: f32,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            sentiment: 0.0,
            emotions: HashMap::new(),
            intensity: 0.5,
            valence: 0.0,
            arousal: 0.5,
        }
    }
}

impl EmotionalState {
    /// Creates a positive emotional state.
    #[must_use]
    pub fn positive(intensity: f32) -> Self {
        Self {
            sentiment: intensity.clamp(0.0, 1.0),
            emotions: HashMap::new(),
            intensity: intensity.clamp(0.0, 1.0),
            valence: intensity.clamp(0.0, 1.0),
            arousal: 0.6,
        }
    }

    /// Creates a negative emotional state.
    #[must_use]
    pub fn negative(intensity: f32) -> Self {
        Self {
            sentiment: -intensity.clamp(0.0, 1.0),
            emotions: HashMap::new(),
            intensity: intensity.clamp(0.0, 1.0),
            valence: -intensity.clamp(0.0, 1.0),
            arousal: 0.6,
        }
    }

    /// Creates a neutral emotional state.
    #[must_use]
    pub fn neutral() -> Self {
        Self::default()
    }

    /// Returns true if the sentiment is positive.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.sentiment > 0.0
    }

    /// Returns true if the sentiment is negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.sentiment < 0.0
    }

    /// Returns true if the sentiment is neutral.
    #[must_use]
    pub fn is_neutral(&self) -> bool {
        self.sentiment.abs() < 0.1
    }

    /// Returns true if this is a high intensity state.
    #[must_use]
    pub fn is_intense(&self) -> bool {
        self.intensity > 0.7
    }

    /// Returns true if this is a calm state.
    #[must_use]
    pub fn is_calm(&self) -> bool {
        self.arousal < 0.3
    }

    /// Returns true if this is an excited state.
    #[must_use]
    pub fn is_excited(&self) -> bool {
        self.arousal > 0.7
    }

    /// Returns the count of detected emotions.
    #[must_use]
    pub fn emotion_count(&self) -> usize {
        self.emotions.len()
    }

    /// Returns true if no emotions are detected.
    #[must_use]
    pub fn has_no_emotions(&self) -> bool {
        self.emotions.is_empty()
    }

    /// Returns the dominant emotion if any.
    #[must_use]
    pub fn dominant_emotion(&self) -> Option<(&String, f32)> {
        self.emotions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k, *v))
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        let mood = if self.is_positive() {
            "positive"
        } else if self.is_negative() {
            "negative"
        } else {
            "neutral"
        };
        format!(
            "EmotionalState: {} (sentiment={:.2}, intensity={:.2})",
            mood, self.sentiment, self.intensity
        )
    }
}

impl std::fmt::Display for EmotionalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Metadata about a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    /// Turn index when the conversation started. Not a wall clock.
    pub started_turn: u64,

    /// Turn index recorded on the last update. Not a wall clock.
    pub last_turn: u64,

    /// Turn count
    pub turn_count: usize,

    /// Conversation topics
    pub topics: Vec<String>,

    /// Conversation quality metrics
    pub quality_metrics: QualityMetrics,

    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
}

/// Quality metrics for a conversation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Average response quality (0.0 to 1.0)
    pub avg_response_quality: f32,

    /// Coherence score (0.0 to 1.0)
    pub coherence_score: f32,

    /// Engagement score (0.0 to 1.0)
    pub engagement_score: f32,

    /// Repetition penalty
    pub repetition_penalty: f32,
}

impl QualityMetrics {
    /// Returns the overall score as an average of all metrics.
    #[must_use]
    pub fn overall_score(&self) -> f32 {
        let base = (self.avg_response_quality + self.coherence_score + self.engagement_score) / 3.0;
        (base - self.repetition_penalty).clamp(0.0, 1.0)
    }

    /// Returns true if quality is high (>0.7).
    #[must_use]
    pub fn is_high_quality(&self) -> bool {
        self.overall_score() > 0.7
    }

    /// Returns true if quality is low (<0.4).
    #[must_use]
    pub fn is_low_quality(&self) -> bool {
        self.overall_score() < 0.4
    }

    /// Returns true if there's significant repetition.
    #[must_use]
    pub fn has_repetition(&self) -> bool {
        self.repetition_penalty > 0.1
    }

    /// Returns the grade as a letter (A, B, C, D, F).
    #[must_use]
    pub fn grade(&self) -> char {
        let score = self.overall_score();
        if score >= 0.9 {
            'A'
        } else if score >= 0.8 {
            'B'
        } else if score >= 0.7 {
            'C'
        } else if score >= 0.6 {
            'D'
        } else {
            'F'
        }
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "QualityMetrics: grade={}, overall={:.2}",
            self.grade(),
            self.overall_score()
        )
    }
}

impl std::fmt::Display for QualityMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Dynamic branching state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BranchingState {
    /// Current branch ID
    pub current_branch: Option<String>,

    /// Available branches
    pub available_branches: Vec<DialogueBranch>,

    /// Branch history
    pub branch_history: Vec<String>,

    /// Branch probability scores
    pub branch_probabilities: HashMap<String, f32>,
}

impl BranchingState {
    /// Returns true if currently on a branch.
    #[must_use]
    pub fn is_on_branch(&self) -> bool {
        self.current_branch.is_some()
    }

    /// Returns true if no branch is active.
    #[must_use]
    pub fn is_on_main(&self) -> bool {
        self.current_branch.is_none()
    }

    /// Returns the number of available branches.
    #[must_use]
    pub fn available_count(&self) -> usize {
        self.available_branches.len()
    }

    /// Returns true if branches are available.
    #[must_use]
    pub fn has_branches(&self) -> bool {
        !self.available_branches.is_empty()
    }

    /// Returns the branch history length.
    #[must_use]
    pub fn history_length(&self) -> usize {
        self.branch_history.len()
    }

    /// Returns the highest probability branch.
    #[must_use]
    pub fn highest_probability_branch(&self) -> Option<(&str, f32)> {
        self.branch_probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(k, v)| (k.as_str(), *v))
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        match &self.current_branch {
            Some(branch) => format!(
                "BranchingState: on '{}', {} available",
                branch,
                self.available_count()
            ),
            None => format!(
                "BranchingState: main path, {} available",
                self.available_count()
            ),
        }
    }
}

impl std::fmt::Display for BranchingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// A dynamic dialogue branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueBranch {
    /// Branch ID
    pub id: String,

    /// Branch condition/trigger
    pub condition: String,

    /// Branch probability
    pub probability: f32,

    /// Branch content
    pub content: DialogueBranchContent,

    /// Branch metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DialogueBranch {
    /// Returns true if this branch has high probability (>0.7).
    #[must_use]
    pub fn is_likely(&self) -> bool {
        self.probability > 0.7
    }

    /// Returns true if this branch has low probability (<0.3).
    #[must_use]
    pub fn is_unlikely(&self) -> bool {
        self.probability < 0.3
    }

    /// Returns the response count in this branch.
    #[must_use]
    pub fn response_count(&self) -> usize {
        self.content.responses.len()
    }

    /// Returns true if this branch has responses.
    #[must_use]
    pub fn has_responses(&self) -> bool {
        !self.content.responses.is_empty()
    }

    /// Returns true if this branch has metadata.
    #[must_use]
    pub fn has_metadata(&self) -> bool {
        !self.metadata.is_empty()
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Branch[{}]: prob={:.2}, {} responses",
            self.id,
            self.probability,
            self.response_count()
        )
    }
}

impl std::fmt::Display for DialogueBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Content of a dialogue branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueBranchContent {
    /// Generated responses
    pub responses: Vec<GeneratedResponse>,

    /// Emotional context
    pub emotional_context: EmotionalState,

    /// Narrative impact
    pub narrative_impact: f32,
}

impl DialogueBranchContent {
    /// Returns the response count.
    #[must_use]
    pub fn response_count(&self) -> usize {
        self.responses.len()
    }

    /// Returns true if there are responses.
    #[must_use]
    pub fn has_responses(&self) -> bool {
        !self.responses.is_empty()
    }

    /// Returns true if this content is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }

    /// Returns true if this has high narrative impact (>0.7).
    #[must_use]
    pub fn is_high_impact(&self) -> bool {
        self.narrative_impact > 0.7
    }

    /// Returns true if this has low narrative impact (<0.3).
    #[must_use]
    pub fn is_low_impact(&self) -> bool {
        self.narrative_impact < 0.3
    }

    /// Returns the first response if available.
    #[must_use]
    pub fn first_response(&self) -> Option<&GeneratedResponse> {
        self.responses.first()
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "BranchContent: {} responses, impact={:.2}",
            self.response_count(),
            self.narrative_impact
        )
    }
}

impl std::fmt::Display for DialogueBranchContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// A generated dialogue response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedResponse {
    /// The response text
    pub text: String,

    /// Quality score
    pub quality_score: f32,

    /// Emotional tone
    pub emotional_tone: EmotionalState,

    /// Confidence level
    pub confidence: f32,

    /// Generation metadata
    pub metadata: ResponseMetadata,
}

impl GeneratedResponse {
    /// Returns true if this is a high quality response (>0.8).
    #[must_use]
    pub fn is_high_quality(&self) -> bool {
        self.quality_score > 0.8
    }

    /// Returns true if this is a low quality response (<0.5).
    #[must_use]
    pub fn is_low_quality(&self) -> bool {
        self.quality_score < 0.5
    }

    /// Returns true if this response has high confidence (>0.8).
    #[must_use]
    pub fn is_confident(&self) -> bool {
        self.confidence > 0.8
    }

    /// Returns true if the response text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Returns the text length.
    #[must_use]
    pub fn text_len(&self) -> usize {
        self.text.len()
    }

    /// Returns the word count.
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    /// Returns a truncated version of the text.
    #[must_use]
    pub fn truncated_text(&self, max_len: usize) -> String {
        if self.text.len() <= max_len {
            self.text.clone()
        } else {
            format!("{}...", &self.text[..max_len.saturating_sub(3)])
        }
    }

    /// Returns the grade based on quality score.
    #[must_use]
    pub fn grade(&self) -> char {
        if self.quality_score >= 0.9 {
            'A'
        } else if self.quality_score >= 0.8 {
            'B'
        } else if self.quality_score >= 0.7 {
            'C'
        } else if self.quality_score >= 0.6 {
            'D'
        } else {
            'F'
        }
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Response: quality={:.2} ({}), confidence={:.2}",
            self.quality_score,
            self.grade(),
            self.confidence
        )
    }
}

impl std::fmt::Display for GeneratedResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Metadata about response generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Generation method used
    pub generation_method: GenerationMethod,

    /// Processing time in milliseconds
    pub processing_time_ms: f32,

    /// Number of retry attempts
    pub retry_attempts: usize,

    /// Context used for generation
    pub context_summary: String,
}

impl ResponseMetadata {
    /// Returns true if generation was fast (<100ms).
    #[must_use]
    pub fn is_fast(&self) -> bool {
        self.processing_time_ms < 100.0
    }

    /// Returns true if generation was slow (>1000ms).
    #[must_use]
    pub fn is_slow(&self) -> bool {
        self.processing_time_ms > 1000.0
    }

    /// Returns true if there were retry attempts.
    #[must_use]
    pub fn had_retries(&self) -> bool {
        self.retry_attempts > 0
    }

    /// Returns true if there were multiple retries.
    #[must_use]
    pub fn had_multiple_retries(&self) -> bool {
        self.retry_attempts > 1
    }

    /// Returns true if this succeeded on first try.
    #[must_use]
    pub fn first_try_success(&self) -> bool {
        self.retry_attempts == 0
    }

    /// Returns true if the context summary is empty.
    #[must_use]
    pub fn has_context(&self) -> bool {
        !self.context_summary.is_empty()
    }

    /// Returns the processing time in seconds.
    #[must_use]
    pub fn processing_time_secs(&self) -> f32 {
        self.processing_time_ms / 1000.0
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Metadata: {} in {:.1}ms ({} retries)",
            self.generation_method, self.processing_time_ms, self.retry_attempts
        )
    }
}

impl std::fmt::Display for ResponseMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Methods used for response generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GenerationMethod {
    /// Pure LLM generation
    LlmGeneration,

    /// Template-based generation
    TemplateBased,

    /// Hybrid approach
    Hybrid,

    /// Graph-based (traditional)
    GraphBased,
}

impl GenerationMethod {
    /// Returns the method name as a string.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::LlmGeneration => "LLM Generation",
            Self::TemplateBased => "Template Based",
            Self::Hybrid => "Hybrid",
            Self::GraphBased => "Graph Based",
        }
    }

    /// Returns true if this uses LLM (pure or hybrid).
    #[must_use]
    pub fn uses_llm(&self) -> bool {
        matches!(self, Self::LlmGeneration | Self::Hybrid)
    }

    /// Returns true if this uses templates (pure or hybrid).
    #[must_use]
    pub fn uses_templates(&self) -> bool {
        matches!(self, Self::TemplateBased | Self::Hybrid)
    }

    /// Returns true if this is the traditional graph-based method.
    #[must_use]
    pub fn is_traditional(&self) -> bool {
        matches!(self, Self::GraphBased)
    }

    /// Returns true if this is a pure method (not hybrid).
    #[must_use]
    pub fn is_pure(&self) -> bool {
        !matches!(self, Self::Hybrid)
    }

    /// Returns all generation methods.
    #[must_use]
    pub fn all() -> &'static [GenerationMethod] {
        &[
            Self::LlmGeneration,
            Self::TemplateBased,
            Self::Hybrid,
            Self::GraphBased,
        ]
    }
}

impl std::fmt::Display for GenerationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Performance metrics for the dialogue system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueMetrics {
    pub total_conversations: u64,
    pub active_conversations: u64,
    pub total_responses_generated: u64,
    pub successful_generations: u64,
    pub failed_generations: u64,
    pub avg_response_time_ms: f32,
    pub avg_response_quality: f32,
    pub emotion_detection_accuracy: f32,
    pub conversation_satisfaction: f32,
}

impl DialogueMetrics {
    /// Returns the success rate (0.0 to 1.0).
    #[must_use]
    pub fn success_rate(&self) -> f32 {
        let total = self.successful_generations + self.failed_generations;
        if total == 0 {
            1.0
        } else {
            self.successful_generations as f32 / total as f32
        }
    }

    /// Returns the failure rate (0.0 to 1.0).
    #[must_use]
    pub fn failure_rate(&self) -> f32 {
        1.0 - self.success_rate()
    }

    /// Returns the success rate as a percentage.
    #[must_use]
    pub fn success_percentage(&self) -> f32 {
        self.success_rate() * 100.0
    }

    /// Returns true if the system is healthy (>90% success rate).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.success_rate() > 0.9
    }

    /// Returns true if the system has degraded performance (50-90% success).
    #[must_use]
    pub fn is_degraded(&self) -> bool {
        let rate = self.success_rate();
        (0.5..=0.9).contains(&rate)
    }

    /// Returns true if the system is unhealthy (<50% success rate).
    #[must_use]
    pub fn is_unhealthy(&self) -> bool {
        self.success_rate() < 0.5
    }

    /// Returns true if there are no conversations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_conversations == 0
    }

    /// Returns true if there are active conversations.
    #[must_use]
    pub fn has_active(&self) -> bool {
        self.active_conversations > 0
    }

    /// Returns the total generations (successful + failed).
    #[must_use]
    pub fn total_generations(&self) -> u64 {
        self.successful_generations + self.failed_generations
    }

    /// Returns the average response time in seconds.
    #[must_use]
    pub fn avg_response_time_secs(&self) -> f32 {
        self.avg_response_time_ms / 1000.0
    }

    /// Returns a health status string.
    #[must_use]
    pub fn health_status(&self) -> &'static str {
        if self.is_healthy() {
            "Healthy"
        } else if self.is_degraded() {
            "Degraded"
        } else {
            "Unhealthy"
        }
    }

    /// Returns a quality grade (A-F).
    #[must_use]
    pub fn quality_grade(&self) -> char {
        if self.avg_response_quality >= 0.9 {
            'A'
        } else if self.avg_response_quality >= 0.8 {
            'B'
        } else if self.avg_response_quality >= 0.7 {
            'C'
        } else if self.avg_response_quality >= 0.6 {
            'D'
        } else {
            'F'
        }
    }

    /// Returns a brief summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "DialogueMetrics: {} convos, {:.1}% success, quality={}",
            self.total_conversations,
            self.success_percentage(),
            self.quality_grade()
        )
    }
}

impl std::fmt::Display for DialogueMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

impl LlmDialogueSystem {
    /// Create a new LLM dialogue system
    pub async fn new(
        llm_client: Arc<dyn LlmClient>,
        rag_pipeline: RagPipeline,
        config: DialogueConfig,
    ) -> Result<Self> {
        let mut template_engine = TemplateEngine::new();
        Self::setup_dialogue_templates(&mut template_engine).await?;

        Ok(Self {
            llm_client,
            rag_pipeline: Arc::new(RwLock::new(rag_pipeline)),
            template_engine: Arc::new(RwLock::new(template_engine)),
            conversations: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(DialogueMetrics::default())),
        })
    }

    /// Start a new conversation
    pub async fn start_conversation(
        &self,
        conversation_id: String,
        participants: Vec<String>,
        initial_context: Option<String>,
    ) -> Result<String> {
        let context_config = ContextConfig {
            max_tokens: self.config.context_config.context_window_size,
            sliding_window_size: self.config.context_config.max_history_turns,
            enable_summarization: self.config.context_config.enable_summarization,
            ..Default::default()
        };

        let history = ConversationHistory::with_llm_client(context_config, self.llm_client.clone());

        let mut metadata = ConversationMetadata {
            started_turn: 0,
            last_turn: 0,
            turn_count: 0,
            topics: Vec::new(),
            quality_metrics: QualityMetrics::default(),
            context: HashMap::new(),
        };

        // Add initial context if provided
        if let Some(context) = initial_context {
            metadata
                .context
                .insert("initial_context".to_string(), context.into());
        }

        let conversation = ActiveConversation {
            id: conversation_id.clone(),
            participants,
            history: Arc::new(history),
            emotional_state: EmotionalState::default(),
            metadata,
            current_graph: None,
            current_node_id: None,
            branching_state: BranchingState::default(),
        };

        {
            let mut conversations = self.conversations.write().await;
            conversations.insert(conversation_id.clone(), Arc::new(Mutex::new(conversation)));
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_conversations += 1;
            metrics.active_conversations += 1;
        }

        Ok(conversation_id)
    }

    /// Propose a line from the injected client.
    ///
    /// This is not a sim tick. Live bytes reach the tick only through [`recorded_line`].
    pub async fn generate_response(
        &self,
        conversation_id: &str,
        speaker_persona: Option<&LlmPersona>,
        player_input: &str,
        context: Option<&str>,
    ) -> Result<GeneratedResponse> {
        let start_time = std::time::Instant::now();

        // Hold this conversation for the whole generate-and-update step.
        // The map lock is dropped before the mutex so other conversations proceed.
        let slot = {
            let conversations = self.conversations.read().await;
            conversations
                .get(conversation_id)
                .cloned()
                .ok_or_else(|| anyhow!("Conversation not found: {}", conversation_id))?
        };
        let mut conversation = slot.lock().await;

        // Add player input to history
        conversation
            .history
            .add_message(Role::User, player_input.to_string())
            .await?;

        // Analyze player input emotion
        let player_emotion = self.analyze_emotion(player_input).await?;

        // Retrieve relevant memories
        let relevant_memories = {
            let rag = self.rag_pipeline.read().await;
            let query = MemoryQuery::text(player_input);
            rag.retrieve_with_query(&query, self.config.context_config.max_relevant_memories)
                .await?
        };

        // Generate response with retries for quality
        let mut response = None;
        let mut attempts = 0;

        while response.is_none() && attempts < self.config.quality_control.max_retry_attempts {
            attempts += 1;

            match self
                .generate_single_response(
                    &conversation,
                    speaker_persona,
                    player_input,
                    context,
                    &player_emotion,
                    &relevant_memories,
                )
                .await
            {
                Ok(generated) => {
                    if self.validate_response_quality(&generated).await? {
                        response = Some(generated);
                    }
                }
                Err(e) => {
                    if attempts >= self.config.quality_control.max_retry_attempts {
                        return Err(e);
                    }
                    // Continue to retry
                }
            }
        }

        let response = response.ok_or_else(|| {
            anyhow!(
                "Failed to generate quality response after {} attempts",
                attempts
            )
        })?;

        // Add response to conversation history
        conversation
            .history
            .add_message(Role::Assistant, response.text.clone())
            .await?;

        // Update conversation state in place. `last_turn` is the turn index, not a clock.
        conversation.metadata.last_turn = conversation.metadata.turn_count as u64;
        conversation.metadata.turn_count += 1;
        conversation.emotional_state =
            self.blend_emotional_states(&conversation.emotional_state, &response.emotional_tone);

        // Store interaction as memory
        let memory_text = format!("Player: {} | Assistant: {}", player_input, response.text);
        {
            let mut rag = self.rag_pipeline.write().await;
            rag.add_memory(memory_text).await?;
        }

        // Update metrics
        let processing_time = start_time.elapsed().as_millis() as f32;
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_responses_generated += 1;
            metrics.successful_generations += 1;

            let total = metrics.successful_generations as f32;
            metrics.avg_response_time_ms =
                (metrics.avg_response_time_ms * (total - 1.0) + processing_time) / total;
            metrics.avg_response_quality =
                (metrics.avg_response_quality * (total - 1.0) + response.quality_score) / total;
        }

        Ok(response)
    }

    /// Generate a single response (internal method)
    async fn generate_single_response(
        &self,
        conversation: &ActiveConversation,
        speaker_persona: Option<&LlmPersona>,
        player_input: &str,
        context: Option<&str>,
        player_emotion: &EmotionalState,
        relevant_memories: &[RetrievedMemory],
    ) -> Result<GeneratedResponse> {
        // Build context-aware prompt
        let prompt = self
            .build_dialogue_prompt(
                conversation,
                speaker_persona,
                player_input,
                context,
                player_emotion,
                relevant_memories,
            )
            .await?;

        // Generate response using LLM
        let raw_response = self.llm_client.complete(&prompt).await?;

        // Clean and validate response
        let cleaned_response = self.clean_response(&raw_response);

        // Analyze response emotion
        let response_emotion = self.analyze_emotion(&cleaned_response).await?;

        // Calculate quality score
        let quality_score = self
            .calculate_response_quality(&cleaned_response, conversation, player_input)
            .await?;

        // Calculate confidence based on various factors
        let confidence =
            self.calculate_confidence(&cleaned_response, quality_score, relevant_memories.len());

        Ok(GeneratedResponse {
            text: cleaned_response,
            quality_score,
            emotional_tone: response_emotion,
            confidence,
            metadata: ResponseMetadata {
                generation_method: GenerationMethod::LlmGeneration,
                processing_time_ms: 0.0, // Will be set by caller
                retry_attempts: 0,       // Will be set by caller
                context_summary: self
                    .summarize_context(conversation, relevant_memories)
                    .await,
            },
        })
    }

    /// Build a dialogue prompt with all context
    async fn build_dialogue_prompt(
        &self,
        conversation: &ActiveConversation,
        speaker_persona: Option<&LlmPersona>,
        player_input: &str,
        context: Option<&str>,
        player_emotion: &EmotionalState,
        relevant_memories: &[RetrievedMemory],
    ) -> Result<String> {
        let mut template_context = TemplateContext::new();
        let set_str = |ctx: &mut TemplateContext, key: &str, value: &str| {
            ctx.set_path(key, astraweave_prompts::context::ContextValue::from(value));
        };

        // Basic dialogue info
        set_str(&mut template_context, "player_input", player_input);
        let turn = conversation.metadata.turn_count.to_string();
        set_str(&mut template_context, "turn_count", &turn);

        // Speaker persona information
        if let Some(persona) = speaker_persona {
            set_str(&mut template_context, "speaker.name", &persona.base.voice);
            set_str(&mut template_context, "speaker.tone", &persona.base.tone);
            set_str(&mut template_context, "speaker.humor", &persona.base.humor);
            set_str(
                &mut template_context,
                "speaker.backstory",
                &persona.base.backstory,
            );
            let mood = format!("{:?}", persona.personality_state.emotional_state);
            set_str(&mut template_context, "speaker.mood", &mood);
            let confidence = persona.personality_state.confidence.to_string();
            set_str(&mut template_context, "speaker.confidence", &confidence);
        }

        // Player emotional state
        let sentiment = player_emotion.sentiment.to_string();
        let intensity = player_emotion.intensity.to_string();
        set_str(
            &mut template_context,
            "player_emotion.sentiment",
            &sentiment,
        );
        set_str(
            &mut template_context,
            "player_emotion.intensity",
            &intensity,
        );

        // Conversation history
        let conversation_context = conversation.history.get_context(1024).await?;
        set_str(
            &mut template_context,
            "conversation_history",
            &conversation_context,
        );

        // Relevant memories
        if !relevant_memories.is_empty() {
            let memory_texts: Vec<String> = relevant_memories
                .iter()
                .map(|m| format!("- {}", m.memory.text))
                .collect();
            let joined = memory_texts.join("\n");
            set_str(&mut template_context, "relevant_memories", &joined);
        }

        // Additional context
        if let Some(ctx) = context {
            set_str(&mut template_context, "additional_context", ctx);
        }

        // Current emotional state of conversation
        let conversation_sentiment = conversation.emotional_state.sentiment.to_string();
        set_str(
            &mut template_context,
            "conversation_sentiment",
            &conversation_sentiment,
        );

        // Use template engine to generate prompt
        let template_engine = self.template_engine.read().await;
        template_engine.render("dialogue_response", &template_context)
    }

    /// Analyze emotion in text
    async fn analyze_emotion(&self, text: &str) -> Result<EmotionalState> {
        if !self.config.emotion_analysis.enable_emotion_detection {
            return Ok(EmotionalState::default());
        }

        // Simple emotion analysis (could be enhanced with dedicated models)
        let positive_words = [
            "happy",
            "joy",
            "excited",
            "great",
            "amazing",
            "wonderful",
            "love",
            "like",
        ];
        let negative_words = [
            "sad",
            "angry",
            "hate",
            "terrible",
            "awful",
            "disappointed",
            "frustrated",
        ];

        let text_lower = text.to_lowercase();
        let words: Vec<&str> = text_lower.split_whitespace().collect();

        let positive_count = positive_words
            .iter()
            .filter(|&&word| words.contains(&word))
            .count() as f32;
        let negative_count = negative_words
            .iter()
            .filter(|&&word| words.contains(&word))
            .count() as f32;
        let total_emotional_words = positive_count + negative_count;

        let sentiment = if total_emotional_words > 0.0 {
            (positive_count - negative_count) / total_emotional_words
        } else {
            0.0
        };

        let intensity = (total_emotional_words / words.len() as f32).min(1.0);
        let valence = sentiment;
        let arousal = intensity;

        // Detect specific emotions (simplified)
        let mut emotions = HashMap::new();
        if positive_count > 0.0 {
            emotions.insert("joy".to_string(), positive_count / words.len() as f32);
        }
        if negative_count > 0.0 {
            emotions.insert("sadness".to_string(), negative_count / words.len() as f32);
        }

        Ok(EmotionalState {
            sentiment,
            emotions,
            intensity,
            valence,
            arousal,
        })
    }

    /// Clean and format response text
    fn clean_response(&self, raw_response: &str) -> String {
        // Remove common LLM artifacts
        let cleaned = raw_response
            .trim()
            .lines()
            .map(|line| line.trim())
            .filter(|line| {
                !line.is_empty() && !line.starts_with("Assistant:") && !line.starts_with("AI:")
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Remove excessive punctuation
        let re = Regex::new(r"[.]{3,}").expect("valid regex");
        let cleaned = re.replace_all(&cleaned, "...");

        // Ensure proper sentence ending
        let cleaned =
            if !cleaned.ends_with('.') && !cleaned.ends_with('!') && !cleaned.ends_with('?') {
                format!("{}.", cleaned)
            } else {
                cleaned.to_string()
            };

        cleaned
    }

    /// Validate response quality
    async fn validate_response_quality(&self, response: &GeneratedResponse) -> Result<bool> {
        if !self.config.quality_control.enable_validation {
            return Ok(true);
        }

        // Check minimum quality score
        if response.quality_score < self.config.quality_control.min_quality_score {
            return Ok(false);
        }

        // Check for repetition
        if self.config.quality_control.enable_repetition_detection
            && self.detect_repetition(&response.text)
        {
            return Ok(false);
        }

        if self.config.quality_control.enable_profanity_filter
            && self.contains_profanity(&response.text)
        {
            return Ok(false);
        }

        Ok(true)
    }

    /// Calculate response quality score
    async fn calculate_response_quality(
        &self,
        response: &str,
        _conversation: &ActiveConversation,
        input: &str,
    ) -> Result<f32> {
        let mut quality_score = 0.7; // Base score

        // Length appropriateness
        let word_count = response.split_whitespace().count();
        if word_count > 5 && word_count < 100 {
            quality_score += 0.1;
        }

        // Relevance to input (simple keyword matching)
        let input_lower = input.to_lowercase();
        let response_lower = response.to_lowercase();
        let input_words: Vec<&str> = input_lower.split_whitespace().collect();
        let response_words: Vec<&str> = response_lower.split_whitespace().collect();
        let common_words = input_words
            .iter()
            .filter(|word| response_words.contains(word))
            .count();
        let relevance = common_words as f32 / input_words.len().max(1) as f32;
        quality_score += relevance * 0.2;

        // Coherence (simple check for complete sentences)
        if response.contains('.') || response.contains('!') || response.contains('?') {
            quality_score += 0.1;
        }

        Ok(quality_score.min(1.0))
    }

    /// Calculate confidence score
    fn calculate_confidence(&self, response: &str, quality_score: f32, memory_count: usize) -> f32 {
        let mut confidence = quality_score;

        // Boost confidence if we have relevant memories
        if memory_count > 0 {
            confidence += (memory_count as f32 * 0.05).min(0.2);
        }

        // Reduce confidence for very short responses
        if response.split_whitespace().count() < 3 {
            confidence -= 0.2;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Detect repetitive content
    fn detect_repetition(&self, text: &str) -> bool {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.len() < 4 {
            return false;
        }

        // Check for repeated phrases
        for window_size in 2..=4 {
            for i in 0..=(words.len() - window_size * 2) {
                let phrase1 = &words[i..i + window_size];
                for j in (i + window_size)..=(words.len() - window_size) {
                    let phrase2 = &words[j..j + window_size];
                    if phrase1 == phrase2 {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Basic profanity check
    fn contains_profanity(&self, text: &str) -> bool {
        let profanity_words = ["damn", "hell", "shit", "fuck"]; // Basic list
        let text_lower = text.to_lowercase();
        profanity_words
            .iter()
            .any(|&word| text_lower.contains(word))
    }

    /// Blend two emotional states
    fn blend_emotional_states(
        &self,
        state1: &EmotionalState,
        state2: &EmotionalState,
    ) -> EmotionalState {
        let blend_factor = 0.3; // How much to blend in the new state

        EmotionalState {
            sentiment: state1.sentiment * (1.0 - blend_factor) + state2.sentiment * blend_factor,
            emotions: state2.emotions.clone(), // Use newer emotions
            intensity: state1.intensity * (1.0 - blend_factor) + state2.intensity * blend_factor,
            valence: state1.valence * (1.0 - blend_factor) + state2.valence * blend_factor,
            arousal: state1.arousal * (1.0 - blend_factor) + state2.arousal * blend_factor,
        }
    }

    /// Summarize context for metadata
    async fn summarize_context(
        &self,
        conversation: &ActiveConversation,
        memories: &[RetrievedMemory],
    ) -> String {
        format!(
            "Turn {}, {} memories, sentiment: {:.2}",
            conversation.metadata.turn_count,
            memories.len(),
            conversation.emotional_state.sentiment
        )
    }

    /// Set up template engine with dialogue templates
    async fn setup_dialogue_templates(engine: &mut TemplateEngine) -> Result<()> {
        // Main dialogue response template
        let dialogue_template = PromptTemplate::new("dialogue_response".to_string(),
        r#"You are {{speaker.name}} engaged in conversation. Your personality: {{speaker.tone}}, humor: {{speaker.humor}}.

{{speaker.backstory}}

Current conversation state:
- Your mood: {{speaker.mood}}
- Your confidence: {{speaker.confidence}}
- Turn: {{turn_count}}
- Conversation sentiment: {{conversation_sentiment}}

Player's emotional state:
- Sentiment: {{player_emotion.sentiment}}
- Intensity: {{player_emotion.intensity}}

{{#if conversation_history}}
Recent conversation:
{{conversation_history}}
{{/if}}

{{#if relevant_memories}}
Relevant memories:
{{relevant_memories}}
{{/if}}

{{#if additional_context}}
Context: {{additional_context}}
{{/if}}

The player says: "{{player_input}}"

Respond as {{speaker.name}} would, considering their personality, the emotional context, and conversation history. Be natural, engaging, and emotionally appropriate:"#.trim().to_string()
    );

        engine.register_template("dialogue_response", dialogue_template)?;

        Ok(())
    }

    /// Get conversation by ID
    pub async fn get_conversation(&self, conversation_id: &str) -> Option<ActiveConversation> {
        let slot = {
            let conversations = self.conversations.read().await;
            conversations.get(conversation_id).cloned()?
        };
        let snapshot = slot.lock().await.clone();
        Some(snapshot)
    }

    /// Get system metrics
    pub async fn get_metrics(&self) -> DialogueMetrics {
        self.metrics.read().await.clone()
    }

    /// End a conversation
    pub async fn end_conversation(&self, conversation_id: &str) -> Result<()> {
        let mut conversations = self.conversations.write().await;
        if conversations.remove(conversation_id).is_some() {
            let mut metrics = self.metrics.write().await;
            metrics.active_conversations = metrics.active_conversations.saturating_sub(1);
            Ok(())
        } else {
            Err(anyhow!("Conversation not found: {}", conversation_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_line_is_a_pure_function_of_the_inputs() {
        let a = recorded_line(7, 3, 1, None, "hello");
        let b = recorded_line(7, 3, 1, None, "hello");
        assert_eq!(a, b);
        assert_eq!(a, "scripted:hello");
        assert_eq!(recorded_line(7, 3, 1, Some("stay"), "hello"), "stay");
        // Seeded and not drawn, so a different tick does not change the text.
        assert_eq!(a, recorded_line(7, 4, 1, None, "hello"));
    }

    #[test]
    fn branch_merge_strategy_matches_the_defined_variants() {
        assert_eq!(BranchMergeStrategy::Contextual.name(), "Contextual");
        assert_eq!(BranchMergeStrategy::Emotional.name(), "Emotional");
        assert_eq!(BranchMergeStrategy::PersonaBased.name(), "Persona-Based");
        assert_eq!(BranchMergeStrategy::None.name(), "None");
        assert!(BranchMergeStrategy::Contextual.is_merging());
        assert!(!BranchMergeStrategy::None.is_merging());
        assert_eq!(BranchMergeStrategy::all().len(), 4);
    }

    #[test]
    fn quality_and_emotion_configs_use_the_live_fields() {
        let quality = QualityControlConfig::default();
        assert!(quality.enable_validation);
        assert!(quality.enable_repetition_detection);
        assert!(quality.enable_profanity_filter);
        let emotion = EmotionAnalysisConfig::default();
        assert!(emotion.enable_emotion_detection);
        assert!(emotion.enable_sentiment_analysis);
        let dialogue = DialogueConfig::default();
        assert_eq!(dialogue.max_response_tokens, 256);
        assert!(dialogue.branching_config.enable_dynamic_branching);
    }

    #[test]
    fn the_sim_tick_does_not_call_generate_response() {
        let plugin = include_str!("../../astraweave-ai/src/ecs_ai_plugin.rs");
        let plugin = plugin.split("mod tests").next().unwrap_or("");
        assert!(plugin.contains("fn sys_spatial_tick"));
        assert!(!plugin.contains("generate_response"));
        assert!(!plugin.contains("LlmDialogueSystem"));
        let spatial = include_str!("../../astraweave-ai/src/htn.rs");
        let spatial = spatial.split("mod tests").next().unwrap_or("");
        assert!(spatial.contains("fn plan_route"));
        assert!(!spatial.contains("generate_response"));
        assert!(!spatial.contains("recorded_line"));
    }

    struct EchoClient;

    #[async_trait::async_trait]
    impl LlmClient for EchoClient {
        async fn complete(&self, _prompt: &str) -> anyhow::Result<String> {
            Ok("Hello there friend today. The ford is quiet.".to_string())
        }
    }

    fn speaker_named(name: &str) -> LlmPersona {
        let mut persona = LlmPersona {
            base: Default::default(),
            llm_config: Default::default(),
            personality_state: Default::default(),
            adaptation: Default::default(),
            prompt_settings: Default::default(),
            memory_profile: Default::default(),
        };
        persona.base.voice = name.to_string();
        persona.base.tone = "calm".to_string();
        persona.base.humor = "dry".to_string();
        persona.base.backstory = "Keeps the ford.".to_string();
        persona
    }

    async fn test_system(config: DialogueConfig) -> LlmDialogueSystem {
        let llm: Arc<dyn LlmClient> = Arc::new(EchoClient);
        let rag = astraweave_rag::RagPipeline::new(
            Arc::new(astraweave_embeddings::MockEmbeddingClient::new()),
            Arc::new(astraweave_rag::VectorStoreWrapper::new(
                astraweave_embeddings::VectorStore::new(384),
            )),
            Some(Arc::clone(&llm)),
            astraweave_rag::RagConfig::default(),
        );
        LlmDialogueSystem::new(llm, rag, config)
            .await
            .unwrap_or_else(|err| panic!("dialogue system: {err}"))
    }

    #[tokio::test]
    async fn speaker_name_appears_in_the_rendered_prompt() {
        let system = test_system(DialogueConfig::default()).await;
        let id = system
            .start_conversation("ford".to_string(), vec!["sera".to_string()], None)
            .await
            .unwrap_or_else(|err| panic!("start: {err}"));
        let stored = system
            .get_conversation(&id)
            .await
            .unwrap_or_else(|| panic!("missing conversation"));
        assert_eq!(stored.metadata.started_turn, 0);
        assert_eq!(stored.metadata.last_turn, 0);
        let persona = speaker_named("Sera Thorn");
        let prompt = system
            .build_dialogue_prompt(
                &stored,
                Some(&persona),
                "hello",
                None,
                &EmotionalState::default(),
                &[],
            )
            .await
            .unwrap_or_else(|err| panic!("prompt: {err}"));
        assert!(
            prompt.contains("Sera Thorn"),
            "speaker name missing from prompt: {prompt}"
        );
    }

    #[tokio::test]
    async fn concurrent_generate_response_keeps_both_turns() {
        let mut config = DialogueConfig::default();
        config.quality_control.enable_validation = false;
        let system = Arc::new(test_system(config).await);
        system
            .start_conversation("ford".to_string(), vec!["sera".to_string()], None)
            .await
            .unwrap_or_else(|err| panic!("start: {err}"));
        let persona = speaker_named("Sera Thorn");
        let first =
            system.generate_response("ford", Some(&persona), "hello there friend today", None);
        let second =
            system.generate_response("ford", Some(&persona), "hello there friend today", None);
        let (left, right) = tokio::join!(first, second);
        left.unwrap_or_else(|err| panic!("first: {err}"));
        right.unwrap_or_else(|err| panic!("second: {err}"));
        let stored = system
            .get_conversation("ford")
            .await
            .unwrap_or_else(|| panic!("missing conversation"));
        assert_eq!(stored.metadata.turn_count, 2);
        assert_eq!(stored.metadata.started_turn, 0);
        assert_eq!(stored.metadata.last_turn, 1);
    }
}
