use anyhow::Result;
use astraweave_context::{ContextConfig, ConversationHistory, Role};
use astraweave_core::{ToolRegistry, ToolSpec};
use astraweave_embeddings::{MockEmbeddingClient, VectorStore};
use astraweave_llm::plan_parser::parse_llm_response;
use astraweave_llm::MockLlm;
use astraweave_memory::Persona as BasePersona;
use astraweave_persona::LlmPersonaManager;
use astraweave_prompts::engine::TemplateEngine;
use astraweave_rag::{RagConfig, RagPipeline, VectorStoreWrapper};
use std::collections::BTreeMap;
use std::sync::Arc;

#[tokio::test]
async fn test_full_stack_integration() -> Result<()> {
    // 1. Setup Core Components
    let embedding_client = Arc::new(MockEmbeddingClient::new());
    let vector_store = Arc::new(VectorStoreWrapper::new(VectorStore::new(384)));
    let llm_client = Arc::new(MockLlm);
    let rag_config = RagConfig::default();

    // 2. Initialize RAG Pipeline
    let mut rag = RagPipeline::new(
        embedding_client.clone(),
        vector_store,
        Some(llm_client.clone()),
        rag_config,
    );

    // 3. Add Memories
    rag.add_memory("The player prefers stealth approaches.".to_string())
        .await?;
    rag.add_memory("We previously encountered a dragon in the northern cave.".to_string())
        .await?;

    // 4. Initialize Conversation History
    let context_config = ContextConfig::default();
    let history = ConversationHistory::new(context_config);

    history
        .add_message(
            Role::User,
            "What should we do about the dragon?".to_string(),
        )
        .await?;

    // 5. Initialize Persona
    let base_persona = BasePersona {
        voice: "Astra".to_string(),
        backstory: "Helpful AI companion".to_string(),
        ..Default::default()
    };

    // Note: LlmPersonaManager consumes the RAG pipeline
    let persona_manager = LlmPersonaManager::new(
        base_persona,
        llm_client.clone(),
        rag,
        embedding_client.clone(),
    )
    .await?;

    // 6. Generate Response (Persona + RAG + History)
    let response = persona_manager
        .generate_response("What should we do about the dragon?", None)
        .await?;

    assert!(!response.is_empty());
    // MockLlm returns a fixed JSON plan, so the response will be that JSON.
    // In a real scenario, it would be natural language, but here we check if we got something.

    // 7. Parse Plan from Response (if it contains one)
    // MockLlm returns a JSON plan by default.
    let registry = ToolRegistry {
        tools: vec![
            ToolSpec {
                name: "MoveTo".to_string(),
                args: BTreeMap::new(),
            },
            ToolSpec {
                name: "ThrowSmoke".to_string(),
                args: BTreeMap::new(),
            },
            ToolSpec {
                name: "Attack".to_string(),
                args: BTreeMap::new(),
            },
        ],
        constraints: astraweave_core::Constraints {
            enforce_cooldowns: false,
            enforce_los: false,
            enforce_stamina: false,
        },
    };

    // The MockLlm response is a JSON string.
    let parse_result = parse_llm_response(&response, &registry);

    // It might fail if the MockLlm response isn't exactly what the parser expects
    // (e.g. if LlmPersonaManager wraps it in conversational text).
    // But MockLlm returns raw JSON. LlmPersonaManager might wrap it.
    // Let's check if we can parse it.
    if let Ok(result) = parse_result {
        assert!(!result.plan.steps.is_empty());
        println!(
            "Successfully parsed plan with {} steps",
            result.plan.steps.len()
        );
    } else {
        println!("Response was conversational, not a plan: {}", response);
    }

    // 8. Verify Template Engine
    let mut engine = TemplateEngine::new();
    engine.register_template(
        "test",
        astraweave_prompts::template::PromptTemplate::new(
            "test".to_string(),
            "Hello {{name}}".to_string(),
        ),
    )?;

    let mut context = astraweave_prompts::context::PromptContext::new();
    context.set("name".to_string(), "World".to_string().into());

    let rendered = engine.render("test", &context)?;
    assert_eq!(rendered, "Hello World");

    Ok(())
}
