# AstraWeave: Comprehensive LLM Integration Master Plan

**Version**: 1.1  
**Created**: January 2025  
**Status**: Planning Refined, Ready for Implementation  
**Target Completion**: Q2-Q3 2025 (16-20 weeks)

---

## Executive Summary

This master plan details a **comprehensive, production-ready LLM integration** across the entire AstraWeave engine, transforming it into the world's first truly **AI-native game engine** where artificial intelligence achieves genuine intelligent gameplay through persistent learning, adaptive behavior, and emergent narratives.

**Vision Alignment**: Deliver on AstraWeave's core promise of "**AI agents as first-class citizens with genuine learning, adaptation, and emergent behavior**" through systematic LLM integration across all 40+ engine crates.

### Current State Analysis

**Strengths** ✅:
- Production-ready LLM orchestrator with streaming, fallbacks, warmup
- Multiple backend support (Mock, Ollama, LocalHTTP, OpenAI-compatible)
- Clean abstraction layer (`LlmClient` trait)
- Tool sandbox security with an explicit `ToolGuard` validation layer
- Integration with deterministic ECS architecture
- Model lifecycle management via a versioned registry
- Low-latency focus (local inference, first-token tracking)

**Critical Gaps** ❌:
- **No Long-Term Memory**: Missing embeddings, RAG, vector search
- **No Context Management**: No conversation history or context windows
- **No Personalization**: No persona-specific prompt engineering
- **Limited Integration**: Only orchestrator uses LLM (8 crates need integration)
- **No Formal Evaluation**: Lacks an automated harness for quality scoring.
- **No Explicit Fail-Safes**: Lacks defined deterministic fallback paths.

**Integration Opportunity**: 9 crates ready for LLM features:
1. `astraweave-persona` - Personality-driven prompts
2. `astraweave-dialogue` - Conversation generation
3. `astraweave-director` - Boss AI adaptation
4. `astraweave-quests` - Dynamic quest generation
5. `astraweave-npc` - Emergent NPC behavior
6. `astraweave-behavior` - LLM-assisted behavior trees
7. `astraweave-observability` - LLM call tracing
8. `astraweave-persistence-ecs` - Conversation history save/load
9. `astraweave-memory` - Long-term learning persistence

---

## Strategic Goals

### Primary Objectives

1. **Persistent Companions That Learn** 🎯
   - Long-term memory via embeddings + RAG
   - Conversation history with context management
   - Personality evolution over sessions
   - Learned behavior adaptation

2. **Adaptive Boss Systems** 🎯
   - LLM-driven tactic evolution
   - Player behavior analysis
   - Difficulty adjustment via reasoning
   - Emergent strategy synthesis

3. **Dynamic Content Generation** 🎯
   - Quest generation with narrative branching
   - Dialogue with emotional intelligence
   - NPC social interactions
   - Behavior tree synthesis

4. **Production-Grade Performance** 🎯
   - <100ms p95 latency for critical calls
   - Graceful degradation on LLM failures
   - Batch inference optimization
   - Token budget management

5. **Comprehensive Observability** 🎯
   - LLM call tracing with metrics
   - Performance dashboards
- Automated evaluation harness for quality assurance
- Deterministic fallbacks for service failures### Success Metrics

**Quantitative**:
- ✅ 9/9 identified crates integrate LLM features
- ✅ <100ms p95 latency for orchestrator calls
- ✅ <500ms p95 for dialogue generation
- ✅ <2s p95 for quest generation
- ✅ 95%+ uptime with graceful degradation
- ✅ Automated evaluation scores (BLEU, ROUGE, custom) improve over time
- ✅ 100% of LLM failures result in a graceful, deterministic fallback

**Qualitative**:
- ✅ Companions demonstrate learning across sessions
- ✅ Boss AI evolves tactics based on player behavior
- ✅ NPCs exhibit emergent social interactions
- ✅ Quests feel personalized and dynamic
- ✅ Dialogue maintains personality consistency

---

## Architecture Overview

### Layered Integration Model (v1.1)

```
┌─────────────────────────────────────────────────────────────┐
│                    Game Layer                               │
│  (Persona, Dialogue, Director, Quests, NPC, Behavior)      │
└──────────────────┬──────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│              Intelligence Layer                             │
│  • Context Management (History, Windows, Token Budget)      │
│  • Prompt Engineering (Templates, Persona-Specific)         │
│  • RAG System (Retrieval, Injection, Memory Consolidation)  │
│  • Observability (Tracing, Metrics, Dashboards)            │
│  • Evaluation Harness (Quality Scoring, Regression Testing) │
└──────────────────┬──────────────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────────────┐
│               Foundation Layer                              │
│  • Embeddings (Vector Storage, Semantic Search)             │
│  • LLM Client (Mock, Ollama, LocalHTTP) [EXISTING]         │
│  • Orchestrator & Scheduler (AI Planning, Async Management) │
│  • ToolGuard & Sandbox (Action Registry, Validation)        │
│  • Model Registry (Versioning, Checksums, Properties)       │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow: Perception → Memory → Reasoning → Action (v1.1)

```
1. PERCEPTION & PLANNING (ECS Stages: PERCEPTION, AI_PLANNING)
   ├─ WorldSnapshot (existing)
   ├─ RAG Retrieval (similar memories)
   ├─ Context Window (recent history)
   ├─ Generate Prompt (using Persona & Templates)
   └─ Dispatch to LlmScheduler (returns Future<LlmResponse>)
              ↓
2. ASYNC LLM INFERENCE (Managed by Tokio Runtime)
   ├─ LLM inference (streaming)
   └─ LlmScheduler tracks completion
              ↓
3. RESPONSE & VALIDATION (ECS Stage: AI_RESPONSE_POLL)
   ├─ Poll LlmScheduler for completed futures
   ├─ Parse & validate response structure
   ├─ ToolGuard: Verify each action against ActionRegistry
   └─ On validation failure, trigger fallback or replan
              ↓
4. ACTION & MEMORY (ECS Stages: SIMULATION, POST_SIMULATION)
   ├─ Execute validated actions via ECS command buffer
   ├─ Store outcomes in ECS components
   ├─ Update conversation history
   └─ Embed new experiences & consolidate memory
```

---

## Phased Implementation Plan

### Phase 1: Foundation (Weeks 1-4)

**Goal**: Establish core infrastructure for long-term memory and context management.

#### Task 1.1: Embeddings Layer (`astraweave-embeddings`) - Week 1-2

**Implementation**:
```rust
// New crate: astraweave-embeddings/src/lib.rs
pub trait EmbeddingClient: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

pub struct LocalEmbeddingClient {
    model: SentenceTransformersModel, // all-MiniLM-L6-v2 (384 dims)
}

pub struct VectorStore {
    embeddings: Vec<(String, Vec<f32>)>, // In-memory for MVP
    index: HnswIndex,                     // Approximate nearest neighbor
}

impl VectorStore {
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        // Cosine similarity, top-k results
    }
}
```

**Deliverables**:
- `astraweave-embeddings` crate with `EmbeddingClient` trait
- Local embedding model (sentence-transformers, 384-dim)
- In-memory vector store with HNSW indexing
- Semantic search (cosine similarity, top-k)
- Unit tests (embedding quality, search accuracy)

**Dependencies**:
- `rust-bert` or `ort` (ONNX Runtime) for model inference
- `hnsw` crate for approximate nearest neighbor
- `nalgebra` for vector operations

**Acceptance Criteria**:
- Embed 100 texts in <1s (batched)
- Search 10k vectors in <10ms (p95)
- 95%+ precision@10 for semantic search

---

#### Task 1.2: Context Management (`astraweave-context`) - Week 2-3

**Implementation**:
```rust
// New crate: astraweave-context/src/lib.rs
pub struct ConversationHistory {
    messages: VecDeque<Message>,
    token_budget: usize,
    tokenizer: Tokenizer, // tiktoken or custom
}

impl ConversationHistory {
    pub fn add(&mut self, role: Role, content: String) {
        // Add message, prune if over budget
    }
    
    pub fn get_context(&self, max_tokens: usize) -> String {
        // Return recent messages within token limit
    }
    
    pub fn summarize_old(&mut self, llm: &dyn LlmClient) {
        // Summarize old messages to compress history
    }
}

pub struct ContextWindow {
    max_tokens: usize,
    current_tokens: usize,
    sliding_window: bool, // vs summarization
}
```

**Deliverables**:
- `astraweave-context` crate with conversation history
- Token counting (tiktoken-compatible)
- Sliding window strategy (FIFO pruning)
- Summarization strategy (LLM-based compression)
- Context budget management (prevent overflow)
- ECS component `CConversationHistory` for persistence
- Unit tests (token counting, pruning logic)

**Dependencies**:
- `tiktoken` or custom tokenizer
- Integration with `astraweave-llm` for summarization

**Acceptance Criteria**:
- Maintain 4096-token context window
- Prune in <1ms (sliding window)
- Summarize in <500ms (LLM-based)
- Zero context overflow errors

---

#### Task 1.3: Prompt Templating (`astraweave-prompts`) - Week 3-4

**Implementation**:
```rust
// New crate: astraweave-prompts/src/lib.rs
pub struct PromptTemplate {
    template: String,
    variables: HashMap<String, String>,
}

impl PromptTemplate {
    pub fn render(&self) -> Result<String> {
        // Mustache-style template rendering
    }
    
    pub fn from_persona(persona: &Persona) -> Self {
        // Generate persona-specific prompts
    }
}

pub struct PromptLibrary {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptLibrary {
    pub fn get(&self, key: &str) -> Option<&PromptTemplate> { }
    
    pub fn optimize(&mut self, llm: &dyn LlmClient) {
        // Prompt optimization via DSPy-style techniques
    }
}
```

**Deliverables**:
- `astraweave-prompts` crate with template engine
- Mustache-style variable substitution
- Persona-specific template generation
- Few-shot example management
- Prompt optimization utilities (optional)
- Unit tests (rendering, variable injection)

**Dependencies**:
- `handlebars` or `tera` for templating
- Integration with `astraweave-persona`

**Acceptance Criteria**:
- Render templates in <1ms
- Support 20+ variables per template
- Load from TOML/JSON files
- Hot-reload support

---

### Phase 2: Core Features (Weeks 5-10)

**Goal**: Integrate LLM into key gameplay systems (persona, dialogue, director).

#### Task 2.1: RAG System (`astraweave-rag`) - Week 5-6

**Implementation**:
```rust
// New crate: astraweave-rag/src/lib.rs
pub struct RagPipeline {
    embedder: Arc<dyn EmbeddingClient>,
    vector_store: Arc<RwLock<VectorStore>>,
    reranker: Option<CrossEncoder>, // Optional reranking
}

impl RagPipeline {
    pub async fn retrieve(&self, query: &str, k: usize) -> Result<Vec<String>> {
        let query_emb = self.embedder.embed(query).await?;
        let results = self.vector_store.read().unwrap().search(&query_emb, k);
        // Optional: Rerank with cross-encoder
        Ok(results.into_iter().map(|(text, _)| text).collect())
    }
    
    pub async fn inject_context(&self, prompt: &str, query: &str) -> Result<String> {
        let memories = self.retrieve(query, 5).await?;
        Ok(format!("{}\n\nRelevant memories:\n{}", prompt, memories.join("\n")))
    }
    
    pub async fn consolidate(&mut self, short_term: &[Memory]) -> Result<()> {
        // Convert short-term memories to long-term embeddings
    }
}

pub struct Memory {
    pub text: String,
    pub timestamp: u64,
    pub importance: f32, // For prioritized consolidation
}
```

**Deliverables**:
- `astraweave-rag` crate with retrieval pipeline
- Query → embeddings → search → rerank flow
- Context injection into prompts
- Memory consolidation (short-term → long-term)
- Forgetting curve (importance-based decay)
- ECS component `CMemoryStore` for persistence
- Unit tests (retrieval accuracy, consolidation)

**Dependencies**:
- `astraweave-embeddings`
- `astraweave-prompts`
- Optional: `sentence-transformers` for cross-encoder

**Acceptance Criteria**:
- Retrieve 5 memories in <20ms (p95)
- 90%+ relevance for retrieved memories
- Consolidate 100 memories in <1s
- Support 10k+ memories per NPC

---

#### Task 2.2: Persona Integration (`astraweave-persona` extensions) - Week 6-7

**Implementation**:
```rust
// Extend astraweave-persona/src/lib.rs
pub struct PersonaLlmConfig {
    pub prompt_template: String,
    pub temperature: f32,
    pub top_p: f32,
    pub personality_factors: HashMap<String, f32>, // e.g., "aggression": 0.8
}

impl Persona {
    pub fn generate_prompt(&self, situation: &str) -> String {
        // Use astraweave-prompts to generate persona-specific prompt
        let template = PromptTemplate::from_persona(self);
        template.render_with_vars(&[
            ("situation", situation),
            ("traits", &self.traits.join(", ")),
            ("mood", &self.current_mood),
        ])
    }
    
    pub fn evolve_from_outcome(&mut self, outcome: &ActionOutcome) {
        // Adjust personality factors based on experience
        if outcome.success {
            self.confidence += 0.05;
        } else {
            self.caution += 0.05;
        }
    }
}
```

**Deliverables**:
- Persona-specific prompt generation
- Trait-based LLM parameter tuning (temperature, top_p)
- Personality evolution from outcomes
- Mood system integration
- Emotional state tracking
- ECS component `CPersonaLlm` for runtime state
- Unit tests (prompt generation, evolution logic)

**Dependencies**:
- `astraweave-prompts`
- `astraweave-llm`

**Acceptance Criteria**:
- Generate persona prompt in <5ms
- Personality factors affect LLM responses
- Mood persistence across sessions
- Traits remain consistent

---

#### Task 2.3: Dialogue System (`astraweave-dialogue` extensions) - Week 7-8

**Implementation**:
```rust
// Extend astraweave-dialogue/src/lib.rs
pub struct DialogueGenerator {
    llm: Arc<dyn LlmClient>,
    rag: Arc<RagPipeline>,
    context: ConversationHistory,
}

impl DialogueGenerator {
    pub async fn generate_response(
        &mut self,
        npc_persona: &Persona,
        player_message: &str,
    ) -> Result<String> {
        // 1. Retrieve relevant memories
        let memories = self.rag.retrieve(player_message, 3).await?;
        
        // 2. Build prompt with context
        let prompt = npc_persona.generate_prompt(&format!(
            "Player said: {}\nContext: {}",
            player_message,
            memories.join(", ")
        ));
        
        // 3. Add conversation history
        let full_prompt = self.context.inject_context(&prompt);
        
        // 4. Generate response
        let response = self.llm.complete(&full_prompt).await?;
        
        // 5. Update history
        self.context.add(Role::User, player_message.to_string());
        self.context.add(Role::Assistant, response.clone());
        
        Ok(response)
    }
    
    pub fn analyze_sentiment(&self, text: &str) -> EmotionalState {
        // Simple sentiment analysis or LLM-based
    }
}
```

**Deliverables**:
- LLM-based dialogue generation with RAG
- Conversation history integration
- Emotional intelligence (sentiment analysis)
- Multi-turn conversation support
- Interrupt/resume functionality
- Dialogue trees with LLM fallback
- ECS component `CDialogueState` for persistence
- Unit tests (response quality, context handling)

**Dependencies**:
- `astraweave-llm`
- `astraweave-rag`
- `astraweave-context`
- `astraweave-persona`

**Acceptance Criteria**:
- Generate response in <500ms (p95)
- Maintain personality consistency
- Remember last 10 conversation turns
- Graceful fallback to scripted dialogue

---

#### Task 2.4: Director AI (`astraweave-director` extensions) - Week 8-9

**Implementation**:
```rust
// Extend astraweave-director/src/lib.rs
pub struct DirectorAI {
    llm: Arc<dyn LlmClient>,
    rag: Arc<RagPipeline>,
    player_model: PlayerBehaviorModel,
}

impl DirectorAI {
    pub async fn adapt_tactics(&mut self, combat_state: &CombatSnapshot) -> Result<TacticPlan> {
        // 1. Analyze player behavior
        let player_analysis = self.player_model.analyze(combat_state);
        
        // 2. Retrieve similar past encounters
        let past_encounters = self.rag.retrieve(&player_analysis, 5).await?;
        
        // 3. Generate prompt
        let prompt = format!(
            "Player analysis: {}\nPast encounters: {}\n\nGenerate boss tactics:",
            player_analysis, past_encounters.join(", ")
        );
        
        // 4. LLM generates tactics
        let tactics_json = self.llm.complete(&prompt).await?;
        let tactics: TacticPlan = serde_json::from_str(&tactics_json)?;
        
        // 5. Store outcome for learning
        self.rag.consolidate(&[Memory {
            text: format!("Tactics: {:?}", tactics),
            timestamp: now(),
            importance: 0.8,
        }]).await?;
        
        Ok(tactics)
    }
    
    pub fn adjust_difficulty(&mut self, player_skill: f32) -> DifficultyModifiers {
        // LLM-based reasoning for difficulty
    }
}

pub struct PlayerBehaviorModel {
    aggression: f32,
    caution: f32,
    skill_level: f32,
    preferred_tactics: Vec<String>,
}
```

**Deliverables**:
- LLM-driven boss tactic adaptation
- Player behavior modeling
- Difficulty adjustment via LLM reasoning
- Past encounter retrieval (RAG)
- Tactic evolution learning
- ECS component `CDirectorState` for runtime state
- Unit tests (tactic generation, difficulty adjustment)

**Dependencies**:
- `astraweave-llm`
- `astraweave-rag`
- `astraweave-prompts`

**Acceptance Criteria**:
- Adapt tactics in <1s (p95)
- Tactics counter player strategies
- Difficulty adjusts dynamically
- Learning from 100+ encounters

---

#### Task 2.5: Quest Generation (`astraweave-quests` extensions) - Week 9-10

**Implementation**:
```rust
// Extend astraweave-quests/src/lib.rs
pub struct QuestGenerator {
    llm: Arc<dyn LlmClient>,
    rag: Arc<RagPipeline>,
    world_state: WorldSnapshot,
}

impl QuestGenerator {
    pub async fn generate_quest(&mut self, context: &QuestContext) -> Result<Quest> {
        // 1. Retrieve player history
        let history = self.rag.retrieve(&context.player_id, 10).await?;
        
        // 2. Build prompt
        let prompt = format!(
            "Player history: {}\nWorld state: {}\nGenerate quest:",
            history.join(", "), serde_json::to_string(&self.world_state)?
        );
        
        // 3. LLM generates quest
        let quest_json = self.llm.complete(&prompt).await?;
        let quest: Quest = serde_json::from_str(&quest_json)?;
        
        // 4. Validate quest structure
        quest.validate(&self.world_state)?;
        
        Ok(quest)
    }
    
    pub fn branch_narrative(&self, player_choice: &Choice) -> Vec<QuestBranch> {
        // Dynamic branching based on player choices
    }
}
```

**Deliverables**:
- LLM-based quest generation
- Narrative branching with player choices
- Quest validation (coherence, feasibility)
- Player history integration (RAG)
- Quest chaining and dependencies
- ECS component `CQuestGenerator` for runtime state
- Unit tests (quest structure, validation)

**Dependencies**:
- `astraweave-llm`
- `astraweave-rag`
- `astraweave-prompts`

**Acceptance Criteria**:
- Generate quest in <2s (p95)
- Quests feel personalized
- 95%+ structural validity
- Support 3-5 quest branches

---

### Phase 3: Advanced Features (Weeks 11-14)

**Goal**: Implement emergent NPC behavior, behavior tree synthesis, and optimization.

#### Task 3.1: NPC Emergent Behavior (`astraweave-npc` extensions) - Week 11-12

**Implementation**:
```rust
// Extend astraweave-npc/src/lib.rs
pub struct NpcAI {
    llm: Arc<dyn LlmClient>,
    rag: Arc<RagPipeline>,
    social_memory: SocialGraph,
}

impl NpcAI {
    pub async fn decide_action(&mut self, situation: &Situation) -> Result<Action> {
        // 1. Retrieve social context
        let relationships = self.social_memory.get_relationships(self.id);
        
        // 2. Retrieve past interactions
        let past_interactions = self.rag.retrieve(&situation.summary(), 5).await?;
        
        // 3. Generate prompt
        let prompt = format!(
            "Relationships: {:?}\nPast: {}\nSituation: {}\nDecide action:",
            relationships, past_interactions.join(", "), situation
        );
        
        // 4. LLM generates action
        let action_json = self.llm.complete(&prompt).await?;
        let action: Action = serde_json::from_str(&action_json)?;
        
        Ok(action)
    }
    
    pub fn update_relationships(&mut self, interaction: &Interaction) {
        // Update social graph based on interactions
        self.social_memory.update(interaction);
    }
}

pub struct SocialGraph {
    relationships: HashMap<(EntityId, EntityId), RelationshipState>,
}

pub struct RelationshipState {
    affinity: f32,      // -1.0 (enemy) to 1.0 (friend)
    trust: f32,
    history: Vec<Interaction>,
}
```

**Deliverables**:
- LLM-driven NPC decision-making
- Social graph with relationships
- Emergent social interactions
- Faction relationship modeling
- Reputation system with LLM reasoning
- ECS component `CNpcAI` for runtime state
- Unit tests (decision quality, social dynamics)

**Dependencies**:
- `astraweave-llm`
- `astraweave-rag`
- `astraweave-persona`

**Acceptance Criteria**:
- Decide action in <300ms (p95)
- NPCs form realistic relationships
- Emergent storylines from interactions
- 100+ NPC social network

---

#### Task 3.2: Behavior Tree Synthesis (`astraweave-behavior` extensions) - Week 12-13

**Implementation**:
```rust
// Extend astraweave-behavior/src/lib.rs
pub struct BehaviorTreeGenerator {
    llm: Arc<dyn LlmClient>,
    library: BehaviorLibrary,
}

impl BehaviorTreeGenerator {
    pub async fn generate_tree(&mut self, goal: &str) -> Result<BehaviorTree> {
        // 1. Retrieve similar behaviors from library
        let similar = self.library.search(goal, 5);
        
        // 2. Generate prompt
        let prompt = format!(
            "Goal: {}\nSimilar behaviors: {}\nGenerate behavior tree (JSON):",
            goal, similar.join(", ")
        );
        
        // 3. LLM generates tree structure
        let tree_json = self.llm.complete(&prompt).await?;
        let tree: BehaviorTree = serde_json::from_str(&tree_json)?;
        
        // 4. Validate tree structure
        tree.validate()?;
        
        // 5. Add to library if successful
        self.library.add(goal, tree.clone());
        
        Ok(tree)
    }
    
    pub fn adapt_tree(&mut self, tree: &BehaviorTree, outcome: &Outcome) -> BehaviorTree {
        // Modify tree based on success/failure
    }
}
```

**Deliverables**:
- LLM-assisted behavior tree generation
- Adaptive behavior based on outcomes
- Behavior library learning
- Tree validation and optimization
- Performance-based pruning
- ECS component `CBehaviorGenerator` for runtime state
- Unit tests (tree generation, adaptation)

**Dependencies**:
- `astraweave-llm`
- `astraweave-behavior`

**Acceptance Criteria**:
- Generate tree in <1s (p95)
- 95%+ structural validity
- Adaptive modification works
- Library with 50+ behaviors

---

#### Task 3.3: Performance Optimization - Week 13-14

**Implementation**:
```rust
// Extend astraweave-llm/src/lib.rs
pub struct BatchInference {
    llm: Arc<dyn LlmClient>,
    queue: Arc<RwLock<Vec<(String, oneshot::Sender<String>)>>>,
    batch_size: usize,
    timeout: Duration,
}

impl BatchInference {
    pub async fn infer(&self, prompt: String) -> Result<String> {
        // Add to queue, wait for batch or timeout
    }
    
    async fn process_batch(&mut self) {
        // Send batch to LLM, distribute responses
    }
}

pub struct PromptCache {
    cache: Arc<RwLock<HashMap<u64, String>>>,
    ttl: Duration,
}

impl PromptCache {
    pub fn get_or_compute(&self, prompt: &str, compute: impl Future<Output = String>) -> String {
        // Cache responses for repeated prompts
    }
}
```

**Deliverables**:
- Batch inference for multiple agents
- Prompt caching (reduce redundant calls)
- Token budget optimization
- Speculative decoding (optional, model-dependent)
- Request prioritization (critical vs background)
- Performance profiling tools
- Unit tests (batching, caching)

**Dependencies**:
- `astraweave-llm`
- `dashmap` for concurrent cache

**Acceptance Criteria**:
- 3x throughput improvement (batching)
- 80%+ cache hit rate (common prompts)
- <5ms cache lookup
- <100ms batch processing overhead

---

### Phase 4: Production Hardening (Weeks 15-16)

**Goal**: Observability, production readiness, and polish.

#### Task 4.1: Observability (`astraweave-observability` extensions) - Week 15

**Implementation**:
```rust
// Extend astraweave-observability/src/lib.rs
pub struct LlmTelemetry {
    traces: Arc<RwLock<Vec<LlmTrace>>>,
    metrics: Arc<RwLock<LlmMetrics>>,
}

pub struct LlmTrace {
    pub request_id: String,
    pub prompt: String,
    pub response: String,
    pub latency_ms: u64,
    pub tokens_prompt: usize,
    pub tokens_response: usize,
    pub cost_usd: f64,
    pub timestamp: u64,
}

pub struct LlmMetrics {
    pub total_calls: u64,
    pub p50_latency_ms: u64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub error_rate: f32,
}

impl LlmTelemetry {
    pub fn record_call(&mut self, trace: LlmTrace) {
        // Record trace, update metrics
    }
    
    pub fn dashboard_data(&self) -> DashboardData {
        // Generate data for UI dashboard
    }
    
    pub fn export_traces(&self, format: ExportFormat) -> Result<String> {
        // Export to JSON, CSV, or OpenTelemetry
    }
}
```

**Deliverables**:
- LLM call tracing (request/response, latency, tokens, cost)
- Performance dashboards (real-time metrics)
- Alert thresholds (latency spikes, errors)
- Export to OpenTelemetry (optional)
- Cost monitoring and budgets
- ECS component `CLlmTelemetry` for persistence
- Unit tests (tracing, metrics calculation)

**Dependencies**:
- `astraweave-observability`
- `prometheus` (optional, for metrics export)

**Acceptance Criteria**:
- Trace all LLM calls with <1ms overhead
- Dashboard updates in real-time
- Alerts trigger on anomalies
- Cost tracking accurate to $0.01

---

#### Task 4.2: A/B Testing Framework - Week 15

**Implementation**:
```rust
// New module: astraweave-llm/src/ab_testing.rs
pub struct ABTestFramework {
    experiments: HashMap<String, Experiment>,
}

pub struct Experiment {
    pub name: String,
    pub control_prompt: String,
    pub variant_prompts: Vec<String>,
    pub assignment_fn: Box<dyn Fn(&str) -> usize>, // User ID -> variant index
}

impl ABTestFramework {
    pub fn assign_variant(&self, experiment: &str, user_id: &str) -> String {
        // Deterministic variant assignment
    }
    
    pub fn record_outcome(&mut self, experiment: &str, variant: usize, outcome: Outcome) {
        // Track success metrics per variant
    }
    
    pub fn analyze(&self, experiment: &str) -> ExperimentResults {
        // Statistical significance testing
    }
}
```

**Deliverables**:
- A/B test framework for prompts/models
- Deterministic variant assignment
- Outcome tracking (success rate, latency)
- Statistical significance testing
- TOML-based experiment configuration
- Unit tests (assignment, analysis)

**Dependencies**:
- `statrs` for statistical tests

**Acceptance Criteria**:
- Run 10+ concurrent experiments
- <1ms variant assignment
- P-value calculation for significance
- Export results to JSON

---

#### Task 4.3: Production Hardening - Week 16

**Implementation**:
```rust
// Extend astraweave-llm/src/lib.rs
pub struct RateLimiter {
    max_requests_per_sec: usize,
    current_requests: Arc<AtomicUsize>,
}

impl RateLimiter {
    pub async fn acquire(&self) -> Result<()> {
        // Block if rate limit exceeded
    }
}

pub struct CircuitBreaker {
    failure_threshold: usize,
    timeout: Duration,
    state: Arc<RwLock<BreakerState>>,
}

impl CircuitBreaker {
    pub async fn call<F>(&self, f: F) -> Result<String>
    where
        F: Future<Output = Result<String>>,
    {
        // Open circuit if too many failures
    }
}
```

**Deliverables**:
- Rate limiting (prevent API overload)
- Circuit breakers for LLM failures
- Backpressure handling (queue limits)
- Graceful degradation to fallbacks
- Retry with exponential backoff
- Timeout tuning per use case
- Integration tests (failure scenarios)

**Dependencies**:
- `tokio` for async

**Acceptance Criteria**:
- Rate limit prevents overload
- Circuit breaker opens after 5 failures
- Backpressure prevents queue overflow
- 99.9%+ uptime with fallbacks

---

## Integration Points

### Crate-by-Crate Integration

| Crate | Integration Type | Priority | Effort |
|-------|-----------------|----------|--------|
| `astraweave-persona` | Prompt generation, personality evolution | **HIGH** | 2 weeks |
| `astraweave-dialogue` | Conversation generation, RAG | **HIGH** | 2 weeks |
| `astraweave-director` | Boss AI, tactic adaptation | **HIGH** | 2 weeks |
| `astraweave-quests` | Quest generation, branching | **MEDIUM** | 2 weeks |
| `astraweave-npc` | Emergent behavior, social graph | **MEDIUM** | 2 weeks |
| `astraweave-behavior` | Behavior tree synthesis | **MEDIUM** | 2 weeks |
| `astraweave-observability` | LLM tracing, metrics | **HIGH** | 1 week |
| `astraweave-persistence-ecs` | Conversation history save/load | **MEDIUM** | 1 week |
| `astraweave-memory` | Long-term memory persistence | **HIGH** | 1 week |

---

## Testing Strategy

### Unit Tests

**Coverage Targets**:
- Core functions: 90%+
- RAG pipeline: 85%+
- Prompt generation: 80%+
- Context management: 90%+

**Test Categories**:
1. **Embeddings**: Quality, search accuracy
2. **RAG**: Retrieval relevance, consolidation
3. **Context**: Token counting, pruning
4. **Prompts**: Rendering, variable injection
5. **Dialogue**: Personality consistency
6. **Director**: Tactic generation
7. **Quests**: Structure validation
8. **NPCs**: Decision quality

### Integration Tests

**Scenarios**:
1. **Persistent Companion**: Learn from 10 sessions
2. **Adaptive Boss**: Evolve tactics over 5 encounters
3. **Dynamic Quest**: Generate 3 quests with branching
4. **NPC Social**: Form 10 relationships
5. **Behavior Trees**: Generate and adapt 5 trees

**Performance Tests**:
- 100 concurrent LLM calls (batching)
- 10k embeddings (search speed)
- 1k memories per NPC (consolidation)
- 100 NPCs with LLM AI (throughput)

### Acceptance Tests

**User Stories**:
- As a **player**, I want companions to remember past conversations
- As a **player**, I want bosses to adapt to my tactics
- As a **player**, I want quests tailored to my choices
- As a **developer**, I want LLM metrics in real-time
- As a **developer**, I want graceful degradation on LLM errors

---

## Risk Assessment & Mitigation

### Technical Risks

**Risk 1: LLM Latency Variability**
- **Impact**: High (user experience)
- **Probability**: Medium
- **Mitigation**:
  - Local inference (Ollama, 7B-12B models)
  - Prompt caching (80%+ hit rate)
  - Batch inference (3x throughput)
  - Graceful fallback to scripted behavior

**Risk 2: Vector Search Scalability**
- **Impact**: Medium (10k+ memories)
- **Probability**: Low
- **Mitigation**:
  - HNSW indexing (sub-linear scaling)
  - Persistent storage (disk-backed)
  - Pruning old memories (forgetting curve)

**Risk 3: Context Window Overflow**
- **Impact**: Medium (conversation quality)
- **Probability**: Medium
- **Mitigation**:
  - Token counting (tiktoken)
  - Sliding window (FIFO pruning)
  - Summarization (LLM-based compression)

**Risk 4: LLM Output Quality**
- **Impact**: High (gameplay quality)
- **Probability**: Medium
- **Mitigation**:
  - Schema validation (JSON structure)
  - Tool sandbox (action validation)
  - Prompt engineering (few-shot examples)
  - A/B testing (optimize prompts)

**Risk 5: Cost Explosion (API calls)**
- **Impact**: Low (local inference)
- **Probability**: Low
- **Mitigation**:
  - Prefer local models (Ollama)
  - Prompt caching (reduce calls)
  - Rate limiting (prevent overload)
  - Cost monitoring (budgets, alerts)

---

## Performance Targets

### Latency Budgets

| Use Case | Target P50 | Target P95 | Target P99 |
|----------|-----------|-----------|-----------|
| **Orchestrator (Planning)** | 50ms | 100ms | 200ms |
| **Dialogue (Response)** | 200ms | 500ms | 1000ms |
| **Director (Tactics)** | 500ms | 1000ms | 2000ms |
| **Quest (Generation)** | 1000ms | 2000ms | 3000ms |
| **NPC (Decision)** | 100ms | 300ms | 500ms |
| **Embeddings (Batch 100)** | 500ms | 1000ms | 1500ms |
| **RAG (Retrieval)** | 10ms | 20ms | 50ms |

### Throughput Targets

- **100 LLM calls/sec** (batched, local inference)
- **1000 embeddings/sec** (batched)
- **10k RAG searches/sec** (HNSW index)
- **100 NPCs with LLM AI** (concurrent)

### Resource Budgets

- **Memory**: <2GB for embeddings + vector store (10k memories per NPC)
- **Disk**: <500MB for conversation history (per player)
- **CPU**: <30% avg (local LLM inference)
- **GPU**: <50% avg (local LLM inference, if available)

---

## Dependencies & Prerequisites

### New Crate Dependencies

**Embeddings**:
- `rust-bert` or `ort` (ONNX Runtime) - Model inference
- `hnsw` - Approximate nearest neighbor
- `nalgebra` - Vector operations

**Context Management**:
- `tiktoken` or custom tokenizer - Token counting

**Prompts**:
- `handlebars` or `tera` - Template engine

**RAG**:
- Optional: `sentence-transformers` - Cross-encoder reranking

**Optimization**:
- `dashmap` - Concurrent hashmap for caching
- `statrs` - Statistical tests for A/B

**Observability**:
- Optional: `prometheus` - Metrics export

### Existing Dependencies

**Already Available**:
- `astraweave-llm` - LLM client abstraction
- `astraweave-ai` - Orchestrator integration
- `astraweave-ecs` - ECS system integration
- `tokio` - Async runtime
- `serde`, `serde_json` - Serialization
- `anyhow` - Error handling

---

## Deployment & Rollout

### Phase 1 Deployment (Foundation)

**Week 4 Deliverables**:
- `astraweave-embeddings` (v0.1.0)
- `astraweave-context` (v0.1.0)
- `astraweave-prompts` (v0.1.0)

**Integration**:
- Add to workspace `Cargo.toml`
- Update `astraweave-llm` to use new crates
- CI validation (unit tests, benchmarks)

**Documentation**:
- API docs (rustdoc)
- Usage examples
- Migration guide (if breaking changes)

### Phase 2 Deployment (Core Features)

**Week 10 Deliverables**:
- `astraweave-rag` (v0.1.0)
- `astraweave-persona` (v0.2.0, extensions)
- `astraweave-dialogue` (v0.2.0, extensions)
- `astraweave-director` (v0.2.0, extensions)
- `astraweave-quests` (v0.2.0, extensions)

**Integration**:
- Feature flags for LLM integration (default off)
- Example updates (hello_companion, unified_showcase)
- CI validation (integration tests, performance)

**Documentation**:
- Feature guides (persona, dialogue, director, quests)
- Performance benchmarks
- Best practices (prompt engineering)

### Phase 3 Deployment (Advanced Features)

**Week 14 Deliverables**:
- `astraweave-npc` (v0.2.0, extensions)
- `astraweave-behavior` (v0.2.0, extensions)
- Optimization features (batching, caching)

**Integration**:
- Feature flags (default on for production)
- Example: NPC town with emergent behavior
- CI validation (stress tests, 100 NPCs)

**Documentation**:
- Advanced guides (NPC AI, behavior trees)
- Performance tuning guide
- Troubleshooting guide

### Phase 4 Deployment (Production)

**Week 16 Deliverables**:
- `astraweave-observability` (v0.2.0, extensions)
- A/B testing framework
- Production hardening (rate limiting, circuit breakers)

**Integration**:
- Production-ready (default enabled)
- Dashboard integration (aw_editor)
- CI validation (failure scenarios, load tests)

**Documentation**:
- Production deployment guide
- Observability guide
- Cost optimization guide

---

## Documentation Plan

### Developer Documentation

**API Documentation** (rustdoc):
- All public APIs documented
- Examples for key functions
- Performance characteristics

**Architecture Guides**:
- LLM integration overview
- RAG system design
- Context management
- Prompt engineering patterns

**Integration Guides**:
- Adding LLM to new crates
- Persona-specific prompts
- Dialogue system usage
- Director AI configuration

### User Documentation

**Feature Guides**:
- Persistent companions
- Adaptive bosses
- Dynamic quests
- NPC emergent behavior

**Configuration Guides**:
- Environment variables
- Prompt templates (TOML)
- Performance tuning
- Cost optimization

**Troubleshooting Guides**:
- Common errors
- Performance issues
- Quality problems
- Debugging tools

---

## Success Criteria

### Technical Success

**Completion Criteria** (Weeks 1-16):
- ✅ 9/9 crates integrate LLM features
- ✅ All unit tests passing (90%+ coverage)
- ✅ All integration tests passing (10 scenarios)
- ✅ Performance targets met (latency, throughput)
- ✅ Zero crashes from LLM errors
- ✅ Production-ready deployment

**Quality Criteria**:
- ✅ Companions remember 100+ interactions
- ✅ Bosses adapt tactics in 5+ encounters
- ✅ Quests feel personalized (user testing)
- ✅ NPCs exhibit emergent behavior
- ✅ Dialogue maintains personality consistency

### Business Success

**User Impact**:
- ✅ Companion engagement +50% (time spent)
- ✅ Boss replayability +30% (repeat encounters)
- ✅ Quest completion rate +20%
- ✅ NPC interactions +40% (organic engagement)

**Developer Experience**:
- ✅ LLM integration guide <1 hour to read
- ✅ Add LLM to new crate <1 day
- ✅ Prompt tuning <1 hour per template
- ✅ Observability dashboard <5 min to understand

**Production Readiness**:
- ✅ 99.9%+ uptime with graceful degradation
- ✅ <$10/month LLM costs (local inference)
- ✅ Zero security incidents (sandbox, validation)
- ✅ <5% performance overhead (vs non-LLM)

---

## Timeline Summary

### Gantt Chart (16 Weeks)

```
Week  1: ████████ Task 1.1 (Embeddings)
Week  2: ████████ Task 1.1 (Embeddings) | ████████ Task 1.2 (Context)
Week  3: ████████ Task 1.2 (Context) | ████████ Task 1.3 (Prompts)
Week  4: ████████ Task 1.3 (Prompts)
Week  5: ████████ Task 2.1 (RAG)
Week  6: ████████ Task 2.1 (RAG) | ████████ Task 2.2 (Persona)
Week  7: ████████ Task 2.2 (Persona) | ████████ Task 2.3 (Dialogue)
Week  8: ████████ Task 2.3 (Dialogue) | ████████ Task 2.4 (Director)
Week  9: ████████ Task 2.4 (Director) | ████████ Task 2.5 (Quests)
Week 10: ████████ Task 2.5 (Quests)
Week 11: ████████ Task 3.1 (NPC)
Week 12: ████████ Task 3.1 (NPC) | ████████ Task 3.2 (Behavior)
Week 13: ████████ Task 3.2 (Behavior) | ████████ Task 3.3 (Optimization)
Week 14: ████████ Task 3.3 (Optimization)
Week 15: ████████ Task 4.1 (Observability) | ████████ Task 4.2 (A/B Testing)
Week 16: ████████ Task 4.3 (Production Hardening)
```

### Milestones

**Month 1 (Weeks 1-4)**: Foundation Complete ✅
- Embeddings, context management, prompt templating
- **Deliverable**: Core infrastructure for memory + context

**Month 2 (Weeks 5-8)**: Core Features 50% ✅
- RAG, persona integration, dialogue system
- **Deliverable**: Persistent companions with learning

**Month 3 (Weeks 9-12)**: Core + Advanced Features ✅
- Director AI, quests, NPC emergent behavior
- **Deliverable**: Adaptive bosses + dynamic content

**Month 4 (Weeks 13-16)**: Production Ready ✅
- Optimization, observability, hardening
- **Deliverable**: Production deployment with monitoring

---

## Next Steps

### Immediate Actions (Week 1)

1. **Create New Crates** (Day 1):
   - `astraweave-embeddings`
   - `astraweave-context`
   - `astraweave-prompts`
   - Add to workspace `Cargo.toml`

2. **Implement Embeddings** (Days 2-5):
   - `EmbeddingClient` trait
   - Local embedding model (sentence-transformers)
   - In-memory vector store
   - HNSW indexing

3. **Write Tests** (Days 6-7):
   - Embedding quality tests
   - Search accuracy tests (precision@10)
   - Performance benchmarks

4. **Documentation** (Ongoing):
   - API docs (rustdoc)
   - Architecture guide
   - Usage examples

### Approval Required

**Before Implementation**:
- [ ] Review master plan (this document)
- [ ] Approve architecture design
- [ ] Approve crate structure
- [ ] Approve performance targets
- [ ] Approve timeline (16 weeks)

**Sign-Off**:
- [ ] Technical lead approval
- [ ] Product owner approval
- [ ] Security review approval

---

## Conclusion

This comprehensive LLM integration plan transforms AstraWeave into a truly AI-native game engine where:

- **Companions learn and adapt** across sessions via embeddings + RAG
- **Bosses evolve tactics** based on player behavior via LLM reasoning
- **Quests feel personalized** with dynamic generation and branching
- **NPCs exhibit emergent behavior** through social graphs and LLM decisions
- **Behavior adapts** via LLM-assisted synthesis and learning

With **16 weeks of systematic implementation**, **9 crates integrated**, and **production-grade observability**, AstraWeave will deliver on its core promise: **"AI agents as first-class citizens with genuine learning, adaptation, and emergent behavior."**

**Ready to begin Phase 1?** 🚀

---

**Document Version**: 1.0  
**Last Updated**: January 2025  
**Next Review**: After Phase 1 completion (Week 4)
