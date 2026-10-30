// Defensive LLM Adapter Example (stub)
// To be used in astraweave-llm/src/llm_adapter.rs or similar

use astraweave_core::byte_prefix;

/// Maximum allowed prompt length
pub const MAX_PROMPT_LENGTH: usize = 4096;

/// Validates LLM input and output, enforces schema, and handles errors robustly.
pub fn safe_llm_invoke(prompt: &str) -> Result<String, String> {
    if prompt.len() > MAX_PROMPT_LENGTH {
        return Err("Prompt too long".into());
    }
    // TODO: Add further input validation (no dangerous tokens, etc)
    let output = mock_llm_call(prompt)?;
    // TODO: Validate output is valid JSON, matches schema, no code execution, etc
    if !output.trim().starts_with('{') {
        return Err("LLM output is not valid JSON".into());
    }
    Ok(output)
}

fn mock_llm_call(prompt: &str) -> Result<String, String> {
    // Placeholder for actual LLM call
    Ok(format!(
        "{{\"plan_id\":\"mock\",\"steps\":[]}} // echo: {}",
        prompt
    ))
}

/// Check if a prompt is within acceptable length
pub fn is_valid_prompt_length(prompt: &str) -> bool {
    prompt.len() <= MAX_PROMPT_LENGTH
}

/// Check if output appears to be valid JSON (basic check)
pub fn is_json_like(output: &str) -> bool {
    output.trim().starts_with('{')
}

/// Sanitize a prompt by trimming whitespace
pub fn sanitize_prompt(prompt: &str) -> String {
    prompt.trim().to_string()
}

/// Truncate prompt to maximum allowed length
pub fn truncate_prompt(prompt: &str) -> String {
    if prompt.len() > MAX_PROMPT_LENGTH {
        byte_prefix(prompt, MAX_PROMPT_LENGTH).to_string()
    } else {
        prompt.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // safe_llm_invoke Tests
    // ====================================================================

    #[test]
    fn test_safe_llm_invoke_success() {
        let result = safe_llm_invoke("Hello, world!");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.starts_with('{'));
    }

    #[test]
    fn test_safe_llm_invoke_empty_prompt() {
        let result = safe_llm_invoke("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_llm_invoke_long_prompt() {
        let long_prompt = "x".repeat(MAX_PROMPT_LENGTH + 1);
        let result = safe_llm_invoke(&long_prompt);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Prompt too long");
    }

    #[test]
    fn test_safe_llm_invoke_exactly_max_length() {
        let prompt = "a".repeat(MAX_PROMPT_LENGTH);
        let result = safe_llm_invoke(&prompt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_llm_invoke_contains_echo() {
        let prompt = "test input";
        let result = safe_llm_invoke(prompt).unwrap();
        assert!(result.contains("echo: test input"));
    }

    #[test]
    fn test_safe_llm_invoke_returns_json() {
        let result = safe_llm_invoke("generate").unwrap();
        assert!(result.contains("plan_id"));
        assert!(result.contains("steps"));
    }

    // ====================================================================
    // is_valid_prompt_length Tests
    // ====================================================================

    #[test]
    fn test_is_valid_prompt_length_empty() {
        assert!(is_valid_prompt_length(""));
    }

    #[test]
    fn test_is_valid_prompt_length_short() {
        assert!(is_valid_prompt_length("Hello"));
    }

    #[test]
    fn test_is_valid_prompt_length_at_limit() {
        let prompt = "x".repeat(MAX_PROMPT_LENGTH);
        assert!(is_valid_prompt_length(&prompt));
    }

    #[test]
    fn test_is_valid_prompt_length_over_limit() {
        let prompt = "x".repeat(MAX_PROMPT_LENGTH + 1);
        assert!(!is_valid_prompt_length(&prompt));
    }

    // ====================================================================
    // is_json_like Tests
    // ====================================================================

    #[test]
    fn test_is_json_like_valid_object() {
        assert!(is_json_like("{\"key\":\"value\"}"));
    }

    #[test]
    fn test_is_json_like_with_whitespace() {
        assert!(is_json_like("  {\"key\":\"value\"}"));
    }

    #[test]
    fn test_is_json_like_array() {
        assert!(!is_json_like("[1, 2, 3]"));
    }

    #[test]
    fn test_is_json_like_plain_text() {
        assert!(!is_json_like("Hello, world!"));
    }

    #[test]
    fn test_is_json_like_empty() {
        assert!(!is_json_like(""));
    }

    #[test]
    fn test_is_json_like_only_whitespace() {
        assert!(!is_json_like("   "));
    }

    // ====================================================================
    // sanitize_prompt Tests
    // ====================================================================

    #[test]
    fn test_sanitize_prompt_no_whitespace() {
        assert_eq!(sanitize_prompt("hello"), "hello");
    }

    #[test]
    fn test_sanitize_prompt_leading_whitespace() {
        assert_eq!(sanitize_prompt("  hello"), "hello");
    }

    #[test]
    fn test_sanitize_prompt_trailing_whitespace() {
        assert_eq!(sanitize_prompt("hello  "), "hello");
    }

    #[test]
    fn test_sanitize_prompt_both_sides() {
        assert_eq!(sanitize_prompt("  hello  "), "hello");
    }

    #[test]
    fn test_sanitize_prompt_preserves_internal_spaces() {
        assert_eq!(sanitize_prompt("  hello world  "), "hello world");
    }

    #[test]
    fn test_sanitize_prompt_empty() {
        assert_eq!(sanitize_prompt(""), "");
    }

    #[test]
    fn test_sanitize_prompt_only_whitespace() {
        assert_eq!(sanitize_prompt("   "), "");
    }

    // ====================================================================
    // truncate_prompt Tests
    // ====================================================================

    #[test]
    fn test_truncate_prompt_short() {
        let prompt = "hello";
        assert_eq!(truncate_prompt(prompt), "hello");
    }

    #[test]
    fn test_truncate_prompt_at_limit() {
        let prompt = "x".repeat(MAX_PROMPT_LENGTH);
        assert_eq!(truncate_prompt(&prompt).len(), MAX_PROMPT_LENGTH);
    }

    #[test]
    fn test_truncate_prompt_over_limit() {
        let prompt = "x".repeat(MAX_PROMPT_LENGTH + 100);
        let result = truncate_prompt(&prompt);
        assert_eq!(result.len(), MAX_PROMPT_LENGTH);
    }

    #[test]
    fn test_truncate_prompt_empty() {
        assert_eq!(truncate_prompt(""), "");
    }

    #[test]
    fn test_truncate_prompt_multibyte_at_boundary() {
        let prompt = format!("{}tail", "é".repeat(2048));
        assert!(prompt.len() > MAX_PROMPT_LENGTH);
        let result = truncate_prompt(&prompt);
        assert!(result.len() <= MAX_PROMPT_LENGTH);
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    #[test]
    fn test_truncate_prompt_emoji_at_boundary() {
        let mut prompt = "x".repeat(MAX_PROMPT_LENGTH - 1);
        prompt.push('🎮');
        prompt.push('z');
        let result = truncate_prompt(&prompt);
        assert!(result.len() <= MAX_PROMPT_LENGTH);
        assert!(std::str::from_utf8(result.as_bytes()).is_ok());
    }

    // ====================================================================
    // Constants Tests
    // ====================================================================

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_max_prompt_length_is_reasonable() {
        assert!(MAX_PROMPT_LENGTH > 0);
        assert!(MAX_PROMPT_LENGTH >= 1024); // At least 1KB
    }

    // ====================================================================
    // Integration Tests
    // ====================================================================

    #[test]
    fn test_sanitize_then_validate() {
        let prompt = "  hello world  ";
        let sanitized = sanitize_prompt(prompt);
        assert!(is_valid_prompt_length(&sanitized));
    }

    #[test]
    fn test_truncate_then_invoke() {
        let long_prompt = "x".repeat(MAX_PROMPT_LENGTH + 100);
        let truncated = truncate_prompt(&long_prompt);
        let result = safe_llm_invoke(&truncated);
        assert!(result.is_ok());
    }

    #[test]
    fn test_full_pipeline() {
        let input = "  Test prompt with whitespace  ";
        let sanitized = sanitize_prompt(input);
        let truncated = truncate_prompt(&sanitized);

        assert!(is_valid_prompt_length(&truncated));
        let result = safe_llm_invoke(&truncated);
        assert!(result.is_ok());
        assert!(is_json_like(&result.unwrap()));
    }
}
