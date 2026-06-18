use astraweave_llm::compression::PromptCompressor;
use astraweave_llm::few_shot::{FewShotExample, FewShotRegistry};
#[cfg(feature = "ollama")]
use astraweave_llm::hermes2pro_ollama::Hermes2ProOllama;
#[cfg(feature = "ollama")]
use astraweave_llm::phi3_ollama::Phi3Ollama;
#[cfg(feature = "ollama")]
use astraweave_llm::qwen3_ollama::{Qwen3Ollama, DEFAULT_QWEN_INSTRUCT_MODEL};

#[test]
fn test_prompt_compressor_ratio() {
    let compressor = PromptCompressor::new();
    let original = "This is a very long prompt that contains a lot of unnecessary words and filler content that can be removed without losing the core meaning of the text.";
    let compressed = compressor.compress(original);

    // Compression should reduce length
    assert!(compressed.len() < original.len());

    // Calculate ratio
    let ratio = compressed.len() as f32 / original.len() as f32;
    println!("Compression ratio: {:.2}", ratio);

    // Should be at least 10% smaller
    assert!(ratio < 0.9);
}

#[test]
fn test_prompt_compressor_semantic_preservation() {
    let compressor = PromptCompressor::new();
    let original = "The enemy is located at coordinates (10, 20) and is holding a rifle.";
    let compressed = compressor.compress(original);

    // Key information should be preserved
    assert!(compressed.contains("enemy"));
    assert!(compressed.contains("10"));
    assert!(compressed.contains("20"));
    assert!(compressed.contains("rifle"));
}

#[test]
fn test_few_shot_token_budgeting() {
    let mut registry = FewShotRegistry::new();

    // Add examples with varying lengths
    registry.add_example(FewShotExample {
        input: "Short prompt".to_string(),
        output: "Short response".to_string(),
        reasoning: "Short reasoning".to_string(),
        tags: vec!["short".to_string()],
    });

    registry.add_example(FewShotExample {
        input: "Medium length prompt with some details".to_string(),
        output: "Medium length response with some details".to_string(),
        reasoning: "Medium reasoning".to_string(),
        tags: vec!["medium".to_string()],
    });

    registry.add_example(FewShotExample {
        input: "Very long prompt with a lot of details and context that takes up many tokens"
            .to_string(),
        output: "Very long response with a lot of details and context that takes up many tokens"
            .to_string(),
        reasoning: "Long reasoning".to_string(),
        tags: vec!["long".to_string()],
    });

    // Request examples with strict token budget
    // Assuming ~4 chars per token
    let examples = registry.get_examples_with_budget(&["short", "medium", "long"], 20);

    // Should only fit the short example
    assert_eq!(examples.len(), 1);
    assert!(examples[0].input.contains("Short"));

    // Request with larger budget
    let examples_large = registry.get_examples_with_budget(&["short", "medium", "long"], 100);

    // Should fit more examples
    assert!(examples_large.len() > 1);
}

#[test]
#[cfg(feature = "ollama")]
fn test_phi3_variants() {
    // Test Fast variant
    let fast = Phi3Ollama::fast();
    assert_eq!(fast.model, "phi3:game");
    assert_eq!(fast.temperature, 0.5);
    assert_eq!(fast.max_tokens, 128);

    // Test Mini variant
    let mini = Phi3Ollama::mini();
    assert_eq!(mini.model, "phi3:3.8b");
    assert_eq!(mini.temperature, 0.5);
    assert_eq!(mini.max_tokens, 128);
}

#[test]
#[cfg(feature = "ollama")]
fn test_hermes2pro_variants() {
    // Test Fast variant (legacy — retained for backward compat)
    let fast = Hermes2ProOllama::fast();
    assert_eq!(fast.model, "adrienbrault/nous-hermes2pro:Q4_K_M");
    assert_eq!(fast.temperature, 0.5);
    assert_eq!(fast.max_tokens, 128);
}

#[test]
#[cfg(feature = "ollama")]
fn test_qwen3_variants() {
    // Test Fast variant
    let fast = Qwen3Ollama::fast();
    assert_eq!(fast.model, DEFAULT_QWEN_INSTRUCT_MODEL);
    assert_eq!(fast.temperature, 0.5);
    assert_eq!(fast.max_tokens, 128);
    assert!(!fast.enable_thinking);
    assert_eq!(fast.context_length, 8192);

    // Test Strategic variant
    let strategic = Qwen3Ollama::strategic();
    assert_eq!(strategic.model, DEFAULT_QWEN_INSTRUCT_MODEL);
    assert_eq!(strategic.temperature, 0.6);
    assert_eq!(strategic.max_tokens, 1024);
    assert!(strategic.enable_thinking);
    assert_eq!(strategic.context_length, 32768);

    // Test Localhost default
    let default = Qwen3Ollama::localhost();
    assert_eq!(default.model, DEFAULT_QWEN_INSTRUCT_MODEL);
    assert!(!default.enable_thinking);
    assert_eq!(default.context_length, 32768);
}
