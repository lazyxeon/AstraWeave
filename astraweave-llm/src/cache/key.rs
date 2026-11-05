// Prompt key generation with stable hashing for cache lookups

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A cache key for LLM prompts
///
/// The key is based on:
/// - Normalized prompt text (whitespace collapsed, trimmed)
/// - Model identifier
/// - Temperature (quantized to 0.1 precision)
/// - Tool registry hash (for tool vocabulary changes)
///
/// Phase 7: Stores normalized_prompt for semantic similarity matching
#[derive(Debug, Clone)]
pub struct PromptKey {
    prompt_hash: u64,
    /// Process-local client partition; never persist or hash into replay state.
    pub client_id: String,
    pub model: String,      // Phase 7: Made public for similarity matching
    pub temperature_q: u32, // Phase 7: Made public for similarity matching (temperature * 100)
    tools_hash: u64,
    /// Phase 7: Normalized prompt text for similarity search (not included in hash/eq)
    pub normalized_prompt: String,
}

// Manual implementations to exclude normalized_prompt from hash/equality
impl Hash for PromptKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.prompt_hash.hash(state);
        self.client_id.hash(state);
        self.model.hash(state);
        self.temperature_q.hash(state);
        self.tools_hash.hash(state);
        // normalized_prompt deliberately excluded from hash
    }
}

impl PartialEq for PromptKey {
    fn eq(&self, other: &Self) -> bool {
        self.prompt_hash == other.prompt_hash
            && self.client_id == other.client_id
            && self.model == other.model
            && self.temperature_q == other.temperature_q
            && self.tools_hash == other.tools_hash
        // normalized_prompt deliberately excluded from equality
    }
}

impl Eq for PromptKey {}

impl PromptKey {
    /// Create a new prompt key
    ///
    /// # Arguments
    /// * `prompt` - The prompt text (will be normalized)
    /// * `client_id` - Stable client identity for cache partitioning
    /// * `model` - The model identifier (e.g., "phi3:medium")
    /// * `temperature` - The temperature parameter
    /// * `tools` - Tool names for hashing (sorted for stability)
    pub fn new(
        prompt: &str,
        client_id: &str,
        model: &str,
        temperature: f32,
        tools: &[&str],
    ) -> Self {
        let normalized = normalize_prompt(prompt);
        let prompt_hash = hash_string(&normalized);
        let tools_hash = hash_tools(tools);
        let temperature_q = (temperature * 100.0).round() as u32;

        Self {
            prompt_hash,
            client_id: client_id.to_string(),
            model: model.to_string(),
            temperature_q,
            tools_hash,
            normalized_prompt: normalized,
        }
    }

    /// Create from raw hash values (for testing)
    #[cfg(test)]
    pub fn from_hashes(
        prompt_hash: u64,
        client_id: &str,
        model: &str,
        temperature_q: u32,
        tools_hash: u64,
    ) -> Self {
        Self {
            prompt_hash,
            client_id: client_id.to_string(),
            model: model.to_string(),
            temperature_q,
            tools_hash,
            normalized_prompt: String::new(), // Test keys don't need normalized text
        }
    }
}

/// Normalize a prompt for stable hashing
///
/// - Trim whitespace
/// - Collapse multiple spaces into single space
/// - Remove volatile sections (timestamps, random seeds, etc.)
fn normalize_prompt(prompt: &str) -> String {
    let mut result = String::with_capacity(prompt.len());
    let mut last_was_space = false;
    let mut in_volatile_section = false;

    for line in prompt.lines() {
        let trimmed = line.trim();

        // Skip volatile sections (markers for data that changes frequently)
        if trimmed.starts_with("<!-- VOLATILE:") {
            in_volatile_section = true;
            continue;
        }
        if trimmed.ends_with("-->") && in_volatile_section {
            in_volatile_section = false;
            continue;
        }
        if in_volatile_section {
            continue;
        }

        // Skip timestamp lines (common in prompts)
        if trimmed.starts_with("Timestamp:") || trimmed.starts_with("Current time:") {
            continue;
        }

        // Add normalized text
        for ch in trimmed.chars() {
            if ch.is_whitespace() {
                if !last_was_space && !result.is_empty() {
                    result.push(' ');
                    last_was_space = true;
                }
            } else {
                result.push(ch);
                last_was_space = false;
            }
        }
        if !result.is_empty() && !last_was_space {
            result.push(' ');
            last_was_space = true;
        }
    }

    result.trim().to_string()
}

/// Hash a string using DefaultHasher
fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Hash tool names (sorted for stability)
fn hash_tools(tools: &[&str]) -> u64 {
    let mut sorted_tools = tools.to_vec();
    sorted_tools.sort_unstable();

    let mut hasher = DefaultHasher::new();
    for tool in sorted_tools {
        tool.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_normalization_whitespace() {
        let prompt1 = "hello   world\n\n  foo   bar  ";
        let prompt2 = "hello world foo bar";

        let norm1 = normalize_prompt(prompt1);
        let norm2 = normalize_prompt(prompt2);

        assert_eq!(norm1, norm2);
        assert_eq!(norm1, "hello world foo bar");
    }

    #[test]
    fn test_prompt_normalization_timestamps() {
        let prompt1 = "Context\nTimestamp: 2025-10-14T10:30:00\nAgent state";
        let prompt2 = "Context\nTimestamp: 2025-10-14T11:45:00\nAgent state";

        let norm1 = normalize_prompt(prompt1);
        let norm2 = normalize_prompt(prompt2);

        // Timestamps should be stripped, making these equal
        assert_eq!(norm1, norm2);
        assert_eq!(norm1, "Context Agent state");
    }

    #[test]
    fn test_prompt_normalization_volatile_sections() {
        let prompt = r#"
        Static content
        <!-- VOLATILE: random_seed -->
        seed: 12345
        <!-- /VOLATILE -->
        More static content
        "#;

        let normalized = normalize_prompt(prompt);
        assert!(!normalized.contains("seed"));
        assert!(!normalized.contains("12345"));
        assert!(normalized.contains("Static content"));
        assert!(normalized.contains("More static content"));
    }

    #[test]
    fn test_key_equality_same_normalized() {
        let key1 = PromptKey::new("hello  world", "client", "model1", 0.7, &["move", "attack"]);
        let key2 = PromptKey::new("hello world", "client", "model1", 0.7, &["move", "attack"]);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_prompt() {
        let key1 = PromptKey::new("hello world", "client", "model1", 0.7, &["move"]);
        let key2 = PromptKey::new("goodbye world", "client", "model1", 0.7, &["move"]);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_model() {
        let key1 = PromptKey::new("hello", "client", "model1", 0.7, &["move"]);
        let key2 = PromptKey::new("hello", "client", "model2", 0.7, &["move"]);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_client_same_prompt_and_model() {
        let key1 = PromptKey::new("hello", "client_a", "model1", 0.7, &["move"]);
        let key2 = PromptKey::new("hello", "client_b", "model1", 0.7, &["move"]);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_inequality_different_temperature() {
        let key1 = PromptKey::new("hello", "client", "model1", 0.7, &["move"]);
        let key2 = PromptKey::new("hello", "client", "model1", 0.8, &["move"]);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_key_temperature_quantization() {
        // Very small temperature differences should hash to same key
        let key1 = PromptKey::new("hello", "client", "model1", 0.701, &["move"]);
        let key2 = PromptKey::new("hello", "client", "model1", 0.699, &["move"]);

        assert_eq!(key1, key2); // Both round to 70
    }

    #[test]
    fn different_clients_and_models_distinct_keys_for_same_prompt() {
        let prompt = "engage hostiles near the objective";
        let tools = &["MoveTo", "Attack"];
        let alpha = PromptKey::new(prompt, "client-a", "model-one", 0.7, tools);
        let beta = PromptKey::new(prompt, "client-b", "model-two", 0.7, tools);
        assert_ne!(alpha, beta);
    }

    #[test]
    fn test_tools_hash_order_independence() {
        let tools1 = &["move", "attack", "defend"];
        let tools2 = &["defend", "move", "attack"];

        let hash1 = hash_tools(tools1);
        let hash2 = hash_tools(tools2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_tools_hash_sensitivity() {
        let tools1 = &["move", "attack"];
        let tools2 = &["move", "defend"];

        let hash1 = hash_tools(tools1);
        let hash2 = hash_tools(tools2);

        assert_ne!(hash1, hash2);
    }
}
