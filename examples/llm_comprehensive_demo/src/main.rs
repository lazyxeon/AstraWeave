/*!
# Comprehensive LLM Integration Demo

This example demonstrates the full LLM integration capabilities of AstraWeave,
showcasing how all the foundation crates work together to create AI-native gaming experiences.

## Features Demonstrated

1. **Memory & Context Management**: Persistent companion memories and conversation history
2. **RAG Pipeline**: Semantic retrieval and context injection
3. **Prompt Templating**: Persona-specific prompt generation
4. **Embeddings**: Vector-based similarity search for memories
5. **Integration**: How LLM systems integrate with game logic

## Usage

```bash
# Run with mock LLM (always works)
cargo run -p llm_comprehensive_demo

# Run with Ollama (requires Ollama server)
cargo run -p llm_comprehensive_demo --features ollama
```
*/

use anyhow::Result;
use std::io::Write;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};

// LLM Foundation
use astraweave_context::{ContextConfig, ConversationHistory, Role};
use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
use astraweave_llm::{LlmClient, MockLlm};
use astraweave_prompts::context::PromptContext as TemplateContext;
use astraweave_prompts::engine::TemplateEngine;
use astraweave_prompts::template::PromptTemplate;
use astraweave_rag::{RagConfig, RagPipeline, VectorStoreWrapper};

// Game Systems
use std::collections::HashMap;

#[derive(Debug, Clone)]
enum PersonalityTrait {
    Wise,
    Mysterious,
    Helpful,
    Patient,
}

#[derive(Debug, Clone)]
enum Mood {
    Curious,
    #[allow(dead_code)]
    Neutral,
}

#[derive(Debug, Clone)]
struct Companion {
    #[allow(dead_code)]
    id: String,
    name: String,
    description: String,
    personality_traits: Vec<PersonalityTrait>,
    current_mood: Mood,
    #[allow(dead_code)]
    background: Option<String>,
    #[allow(dead_code)]
    goals: Vec<String>,
    #[allow(dead_code)]
    relationships: HashMap<String, String>,
    #[allow(dead_code)]
    memory_keywords: Vec<String>,
    #[allow(dead_code)]
    behavior_modifiers: HashMap<String, String>,
}

#[cfg(feature = "ollama")]
use astraweave_llm::OllamaChatClient;

/// Main demo application
struct LlmComprehensiveDemo {
    /// LLM client for generation
    llm_client: Arc<dyn LlmClient>,

    /// RAG pipeline for memory retrieval
    rag_pipeline: RagPipeline,

    /// Conversation history manager
    conversation_history: ConversationHistory,

    /// Prompt template engine
    template_engine: TemplateEngine,

    /// Companion persona
    companion: Companion,
}

impl LlmComprehensiveDemo {
    /// Initialize the demo with all LLM components
    pub async fn new() -> Result<Self> {
        println!("🚀 Initializing AstraWeave LLM Integration Demo");
        println!("================================================");

        // 1. Initialize LLM client
        println!("📡 Setting up LLM client...");
        let llm_client = Self::create_llm_client().await?;

        // 2. Initialize embeddings and vector store
        println!("🧠 Setting up memory and embeddings...");
        let embedding_client = Arc::new(MockEmbeddingClient::new());
        let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));

        // 3. Initialize RAG pipeline
        println!("🔍 Setting up RAG pipeline...");
        let rag_config = RagConfig::default();
        let rag_pipeline = RagPipeline::new(
            embedding_client,
            vector_store,
            Some(llm_client.clone()),
            rag_config,
        );

        // 4. Initialize conversation history
        println!("💬 Setting up conversation history...");
        let context_config = ContextConfig {
            max_tokens: 4096,
            sliding_window_size: 20,
            enable_summarization: true,
            ..Default::default()
        };
        let conversation_history =
            ConversationHistory::with_llm_client(context_config, llm_client.clone());

        // 5. Initialize prompt template engine
        println!("📝 Setting up prompt templates...");
        let mut template_engine = TemplateEngine::new();
        Self::setup_templates(&mut template_engine)?;

        // 6. Create companion persona
        println!("🤖 Creating AI companion...");
        let companion = Self::create_companion_persona();

        println!("✅ All systems initialized successfully!\n");

        Ok(Self {
            llm_client,
            rag_pipeline,
            conversation_history,
            template_engine,
            companion,
        })
    }

    /// Create appropriate LLM client based on features
    async fn create_llm_client() -> Result<Arc<dyn LlmClient>> {
        #[cfg(feature = "ollama")]
        {
            // Try to connect to Ollama
            let ollama_url = astraweave_llm::loopback_model_url(
                std::env::var("OLLAMA_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
                false,
            );
            let ollama_model =
                std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama2".to_string());

            println!("🔗 Attempting to connect to Ollama at {}", ollama_url);
            let ollama_client = OllamaChatClient::new(ollama_url, ollama_model).expect("model client");

            // Test connection with warmup
            match ollama_client.warmup(10).await {
                Ok(_) => {
                    println!("✅ Connected to Ollama successfully");
                    return Ok(Arc::new(ollama_client));
                }
                Err(e) => {
                    println!("⚠️  Failed to connect to Ollama: {}", e);
                    println!("🔄 Falling back to MockLlm");
                }
            }
        }

        // Use MockLlm as fallback
        println!("🎭 Using MockLlm for demonstration");
        Ok(Arc::new(MockLlm))
    }

    /// Set up prompt templates for different scenarios
    fn setup_templates(engine: &mut TemplateEngine) -> Result<()> {
        // Companion dialogue template
        let companion_template = PromptTemplate::new("companion_dialogue".to_string(),
            r#"You are {{companion.name}}, a {{companion.role}} with the following traits:
{{#each companion.personality_traits}}
- {{this}}
{{/each}}

Current mood: {{companion.mood}}
Personality description: {{companion.description}}

Your conversation history:
{{conversation_context}}

{{#if relevant_memories}}
Relevant memories from your experiences:
{{relevant_memories}}
{{/if}}

The player says: "{{user_input}}"

Respond as {{companion.name}} would, staying true to your personality and past experiences. Be helpful, engaging, and remember what has been discussed before."#.trim().to_string()
        );

        engine.register_template("companion_dialogue", companion_template)?;

        // Memory summarization template
        let memory_template = PromptTemplate::new("memory_summary".to_string(),
            "Summarize this game experience in 1-2 sentences, focusing on key events and outcomes:\n{{experience_text}}".to_string()
        );

        engine.register_template("memory_summary", memory_template)?;

        Ok(())
    }

    /// Create the AI companion's persona
    fn create_companion_persona() -> Companion {
        Companion {
            id: "ai_companion".to_string(),
            name: "Luna".to_string(),
            description: "A wise and mysterious AI companion with deep knowledge of magic and ancient lore. Luna has accompanied many adventurers and has learned much about the world and its inhabitants.".to_string(),
            personality_traits: vec![
                PersonalityTrait::Wise,
                PersonalityTrait::Mysterious,
                PersonalityTrait::Helpful,
                PersonalityTrait::Patient,
            ],
            current_mood: Mood::Curious,
            background: Some("Luna was created by the ancient mages as a guide for worthy adventurers. Over centuries, she has accumulated vast knowledge and experience.".to_string()),
            goals: vec!["Help the player on their quest".to_string(), "Learn about new experiences".to_string()],
            relationships: HashMap::new(),
            memory_keywords: vec!["magic".to_string(), "adventure".to_string(), "wisdom".to_string()],
            behavior_modifiers: HashMap::new(),
        }
    }

    /// Run the interactive demo
    pub async fn run_interactive_demo(&mut self) -> Result<()> {
        println!("🎮 Welcome to the AstraWeave LLM Integration Demo!");
        println!("===================================================");
        println!();
        println!("Meet Luna, your AI companion powered by advanced LLM integration:");
        println!("- 🧠 Persistent memory of your conversations");
        println!("- 🔍 Context-aware responses using RAG");
        println!("- 🎭 Consistent personality and mood");
        println!("- 📝 Dynamic prompt generation");
        println!();
        println!("Type 'help' for commands, 'quit' to exit");
        println!();

        // Add initial system context
        self.conversation_history
            .add_message(
                Role::System,
                format!(
                    "You are Luna, an AI companion. {}",
                    self.companion.description
                ),
            )
            .await?;

        // Add some initial memories to demonstrate RAG
        self.populate_initial_memories().await?;

        // Interactive loop
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        loop {
            print!("You: ");
            std::io::stdout().flush().unwrap();

            if let Some(input) = lines.next_line().await? {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                match input.to_lowercase().as_str() {
                    "quit" | "exit" => {
                        println!("👋 Goodbye! Thanks for exploring AstraWeave's LLM capabilities!");
                        break;
                    }
                    "help" => {
                        self.show_help();
                        continue;
                    }
                    "status" => {
                        self.show_status().await;
                        continue;
                    }
                    "memories" => {
                        self.show_recent_memories().await?;
                        continue;
                    }
                    _ => {
                        // Process the user input
                        match self.process_user_input(input).await {
                            Ok(response) => {
                                println!("Luna: {}", response);
                                println!();
                            }
                            Err(e) => {
                                println!("❌ Error: {}", e);
                                println!("🔄 Luna falls back to basic response mode");
                                println!("Luna: I apologize, I'm having trouble accessing my full capabilities right now. Could you try rephrasing that?");
                                println!();
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process user input through the full LLM pipeline
    async fn process_user_input(&mut self, input: &str) -> Result<String> {
        let start_time = std::time::Instant::now();

        // 1. Add user message to conversation history
        self.conversation_history
            .add_message(Role::User, input.to_string())
            .await?;

        // 2. Retrieve relevant memories using RAG
        let relevant_memories = self.rag_pipeline.retrieve(input, 3).await?;

        // 3. Get conversation context
        let conversation_context = self.conversation_history.get_context(2048).await?;

        // 4. Build prompt using template engine
        let mut template_context = TemplateContext::new();
        template_context.set(
            "companion.name".to_string(),
            self.companion.name.to_string().into(),
        );
        template_context.set("companion.role".to_string(), "AI Companion".into());
        template_context.set(
            "companion.mood".to_string(),
            format!("{:?}", self.companion.current_mood).into(),
        );
        template_context.set(
            "companion.description".to_string(),
            self.companion.description.to_string().into(),
        );
        template_context.set(
            "companion.personality_traits".to_string(),
            self.companion
                .personality_traits
                .iter()
                .map(|t| format!("{:?}", t))
                .collect::<Vec<_>>()
                .join(", ")
                .into(),
        );
        template_context.set("user_input".to_string(), input.to_string().into());
        template_context.set(
            "conversation_context".to_string(),
            conversation_context.into(),
        );

        // Add relevant memories if found
        if !relevant_memories.is_empty() {
            let memories_text: Vec<String> = relevant_memories
                .iter()
                .map(|m| format!("- {}", m.memory.text))
                .collect();
            template_context.set(
                "relevant_memories".to_string(),
                memories_text.join("\n").into(),
            );
        }

        let prompt = self
            .template_engine
            .render("companion_dialogue", &template_context)?;

        // 5. Generate response using LLM
        let response = self.llm_client.complete(&prompt).await?;

        // 6. Add assistant response to conversation history
        self.conversation_history
            .add_message(Role::Assistant, response.clone())
            .await?;

        // 7. Store this interaction as a memory
        let memory_text = format!("Player: {} | Luna: {}", input, response);
        self.rag_pipeline.add_memory(memory_text).await?;

        let processing_time = start_time.elapsed();
        println!("⚡ Processing time: {}ms", processing_time.as_millis());

        Ok(response)
    }

    /// Populate initial memories for demonstration
    async fn populate_initial_memories(&mut self) -> Result<()> {
        let initial_memories = vec![
            "Luna helped the player learn basic fire magic in the enchanted forest",
            "The player discovered a hidden treasure chest with Luna's guidance",
            "Luna shared ancient knowledge about the crystal caves",
            "Player and Luna defeated a shadow demon together using combined magic",
            "Luna taught the player about the history of the mystic towers",
        ];

        println!("📚 Adding initial memories to Luna's knowledge base...");
        for memory in initial_memories {
            self.rag_pipeline.add_memory(memory.to_string()).await?;
        }

        Ok(())
    }

    /// Show available commands
    fn show_help(&self) {
        println!("🆘 Available Commands:");
        println!("  help     - Show this help message");
        println!("  status   - Show system status and metrics");
        println!("  memories - Show recent memories");
        println!("  quit     - Exit the demo");
        println!();
        println!("💡 Just type anything else to chat with Luna!");
        println!();
    }

    /// Show system status and metrics
    async fn show_status(&self) {
        println!("📊 System Status:");
        println!("=================");

        // RAG metrics
        let rag_metrics = self.rag_pipeline.get_metrics();
        println!("🔍 RAG Pipeline:");
        println!("  Total memories: {}", rag_metrics.total_memories_stored);
        println!("  Total queries: {}", rag_metrics.total_queries);
        println!(
            "  Cache hit rate: {:.1}%",
            rag_metrics.cache_hit_rate * 100.0
        );

        // Conversation metrics
        let conv_metrics = self.conversation_history.get_metrics();
        println!("💬 Conversation:");
        println!("  Total messages: {}", conv_metrics.total_messages);
        println!("  Current tokens: {}", conv_metrics.current_tokens);
        println!("  Utilization: {:.1}%", conv_metrics.utilization * 100.0);

        // Companion status
        println!("🤖 Luna Status:");
        println!("  Mood: {:?}", self.companion.current_mood);
        println!(
            "  Active traits: {}",
            self.companion.personality_traits.len()
        );

        println!();
    }

    /// Show recent memories
    async fn show_recent_memories(&self) -> Result<()> {
        let recent_memories = self.rag_pipeline.retrieve("recent experiences", 5).await?;

        println!("🧠 Luna's Recent Memories:");
        println!("==========================");

        if recent_memories.is_empty() {
            println!("  No memories found (this is normal for MockLlm)");
        } else {
            for (i, memory) in recent_memories.iter().enumerate() {
                println!(
                    "  {}. [Score: {:.2}] {}",
                    i + 1,
                    memory.similarity_score,
                    memory.memory.text
                );
            }
        }

        println!();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize and run the demo
    let mut demo = LlmComprehensiveDemo::new().await?;
    demo.run_interactive_demo().await?;

    Ok(())
}
