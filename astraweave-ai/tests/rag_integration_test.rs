// Parked: `astraweave-ai` does not depend on `astraweave-embeddings`, and
// `persona` is not a module of this crate. `feature = "rag"` does not make
// this integration test buildable.
#![cfg(any())]

use astraweave_ai::{LlmPersonaManager, PersonaConfig};
use astraweave_embeddings::MockEmbeddingClient;
use std::sync::Arc;

#[tokio::test]
async fn test_rag_integration_lifecycle() {
    // 1. Setup
    let client = Arc::new(MockEmbeddingClient::new());
    let config = PersonaConfig {
        name: "TestBot".to_string(),
        description: "A test bot".to_string(),
        ..Default::default()
    };
    let manager = LlmPersonaManager::new(config, client);

    // 2. Ingestion (Add Memory)
    manager
        .add_memory("The secret code is 12345.".to_string(), 1.0)
        .await
        .expect("Failed to add memory 1");

    manager
        .add_memory("The weather is sunny.".to_string(), 0.5)
        .await
        .expect("Failed to add memory 2");

    // 3. Retrieval (Get Context)
    let context = manager
        .get_context("What is the secret code?")
        .await
        .expect("Failed to retrieve context");

    println!("Retrieved Context:\n{}", context);

    // 4. Verification
    // Since we use MockEmbeddingClient, embeddings are random/deterministic based on implementation.
    // However, MockEmbeddingClient usually returns a fixed vector or hash-based vector.
    // If it's hash-based, "secret code" query should be somewhat similar to "secret code" doc.
    // For this test, we mainly verify the pipeline doesn't crash and returns *something*.

    // 5. Maintenance
    manager.maintenance().expect("Maintenance failed");
}
