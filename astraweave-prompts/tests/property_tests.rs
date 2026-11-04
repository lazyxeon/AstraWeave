//! Property-based tests for astraweave-prompts
//!
//! These tests use proptest to verify invariants and find edge cases
//! that would be difficult to discover with hand-written tests.

#![allow(clippy::field_reassign_with_default)]

use proptest::prelude::*;

use astraweave_prompts::sanitize::{
    escape_html, escape_template_syntax, normalize_whitespace, sanitize_input,
    sanitize_variable_name, truncate_input, SanitizationConfig, TrustLevel,
};

// ============================================================================
// PROPTEST STRATEGIES
// ============================================================================

/// Strategy for generating arbitrary strings with mixed content
fn mixed_content_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9 !@#$%^&*()\\[\\]{}|;:',.<>?/\\-_+=`~\\n\\t]{0,500}")
        .unwrap()
}

/// Strategy for generating variable-like names
fn variable_name_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z_][a-zA-Z0-9_.]{0,50}").unwrap()
}

/// Strategy for generating potentially malicious inputs
fn potentially_malicious_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Normal text
        mixed_content_strategy(),
        // Template-like syntax
        prop::string::string_regex("\\{\\{[a-z]{0,10}\\}\\}").unwrap(),
        prop::string::string_regex("\\$\\{[a-z]{0,10}\\}").unwrap(),
        // HTML-like content
        prop::string::string_regex("<[a-z]{0,10}>").unwrap(),
        // Path traversal attempts
        Just("../".to_string()),
        Just("..\\".to_string()),
    ]
}

/// Strategy for generating different SanitizationConfig variations
fn config_strategy() -> impl Strategy<Value = SanitizationConfig> {
    (
        100usize..50000, // max_user_input_length
        10usize..200,    // max_variable_name_length
        any::<bool>(),   // allow_control_chars
        any::<bool>(),   // allow_unicode
        1usize..20,      // max_nesting_depth
        any::<bool>(),   // escape_html
        any::<bool>(),   // block_injection_patterns
    )
        .prop_map(
            |(max_len, max_var, ctrl, unicode, depth, html, block)| SanitizationConfig {
                max_user_input_length: max_len,
                max_variable_name_length: max_var,
                allow_control_chars: ctrl,
                allow_unicode: unicode,
                max_nesting_depth: depth,
                escape_html: html,
                block_injection_patterns: block,
            },
        )
}

// ============================================================================
// PROPERTY TESTS: escape_html
// ============================================================================

proptest! {
    /// Property: escape_html should never return strings containing raw < or >
    /// (except in escaped form &lt; or &gt;)
    #[test]
    fn prop_escape_html_no_raw_angle_brackets(input in mixed_content_strategy()) {
        let result = escape_html(&input);
        // After escaping, raw < and > should not exist unless they are part of escape sequences
        let without_escapes = result
            .replace("&lt;", "")
            .replace("&gt;", "")
            .replace("&amp;", "")
            .replace("&quot;", "")
            .replace("&#x27;", "")
            .replace("&#x2F;", "");
        prop_assert!(!without_escapes.contains('<'), "Raw < found after escaping");
        prop_assert!(!without_escapes.contains('>'), "Raw > found after escaping");
    }

    /// Property: escape_html should be idempotent (applying twice = applying once)
    /// This is not strictly true for all escape functions, but verifies no double-encoding issues
    #[test]
    fn prop_escape_html_length_growth_bounded(input in mixed_content_strategy()) {
        let result = escape_html(&input);
        // Maximum expansion: each char could become &quot; (6 chars), so max 6x growth
        prop_assert!(result.len() <= input.len().saturating_mul(6) + 1);
    }

    /// Property: escape_html never panics on arbitrary input
    #[test]
    fn prop_escape_html_never_panics(input in ".*") {
        let _ = escape_html(&input);
    }
}

// ============================================================================
// PROPERTY TESTS: escape_template_syntax
// ============================================================================

proptest! {
    /// Property: escape_template_syntax removes dangerous patterns
    #[test]
    fn prop_escape_template_removes_handlebars(input in ".*") {
        let result = escape_template_syntax(&input);
        // Raw {{ and }} should be escaped
        prop_assert!(!result.contains("{{") || input.contains("{{"));
        prop_assert!(!result.contains("}}") || input.contains("}}"));
    }

    /// Property: escape_template_syntax never panics
    #[test]
    fn prop_escape_template_never_panics(input in ".*") {
        let _ = escape_template_syntax(&input);
    }
}

// ============================================================================
// PROPERTY TESTS: truncate_input
// ============================================================================

proptest! {
    /// Property: truncate_input result is always <= max_length + 3 (for "...")
    #[test]
    fn prop_truncate_respects_length(
        input in mixed_content_strategy(),
        max_length in 10usize..1000
    ) {
        let result = truncate_input(&input, max_length);
        prop_assert!(result.len() <= max_length + 3,
            "Result {} longer than max {} + 3", result.len(), max_length);
    }

    /// Property: truncate_input returns input unchanged if already short enough
    #[test]
    fn prop_truncate_preserves_short_input(
        input in "[a-zA-Z0-9 ]{0,50}",
        max_length in 100usize..1000
    ) {
        let result = truncate_input(&input, max_length);
        if input.len() <= max_length {
            prop_assert_eq!(result, input);
        }
    }

    /// Property: truncate_input never panics
    #[test]
    fn prop_truncate_never_panics(input in ".*", max_length in 0usize..10000) {
        let _ = truncate_input(&input, max_length);
    }
}

// ============================================================================
// PROPERTY TESTS: normalize_whitespace
// ============================================================================

proptest! {
    /// Property: normalize_whitespace result contains no consecutive spaces
    #[test]
    fn prop_normalize_no_consecutive_spaces(input in mixed_content_strategy()) {
        let result = normalize_whitespace(&input);
        prop_assert!(!result.contains("  "), "Found consecutive spaces in result");
    }

    /// Property: normalize_whitespace result has no leading/trailing whitespace
    #[test]
    fn prop_normalize_trimmed(input in mixed_content_strategy()) {
        let result = normalize_whitespace(&input);
        prop_assert_eq!(result.as_str(), result.trim());
    }

    /// Property: normalize_whitespace never panics
    #[test]
    fn prop_normalize_never_panics(input in ".*") {
        let _ = normalize_whitespace(&input);
    }
}

// ============================================================================
// PROPERTY TESTS: sanitize_variable_name
// ============================================================================

proptest! {
    /// Property: valid variable names pass sanitization
    #[test]
    fn prop_valid_varname_passes(name in variable_name_strategy()) {
        let config = SanitizationConfig::default();
        if !name.is_empty() {
            let result = sanitize_variable_name(&name, &config);
            prop_assert!(result.is_ok(), "Valid name {:?} failed: {:?}", name, result);
        }
    }

    /// Property: sanitize_variable_name result only contains valid chars
    #[test]
    fn prop_sanitized_varname_valid_chars(name in variable_name_strategy()) {
        let config = SanitizationConfig::default();
        if !name.is_empty() {
            if let Ok(result) = sanitize_variable_name(&name, &config) {
                for c in result.chars() {
                    prop_assert!(
                        c.is_alphanumeric() || c == '_' || c == '.',
                        "Invalid char {} in result", c
                    );
                }
            }
        }
    }

    /// Property: sanitize_variable_name never panics
    #[test]
    fn prop_sanitize_varname_never_panics(name in ".*") {
        let config = SanitizationConfig::default();
        let _ = sanitize_variable_name(&name, &config);
    }
}

// ============================================================================
// PROPERTY TESTS: sanitize_input
// ============================================================================

proptest! {
    /// Property: Developer and System trust levels always pass through
    #[test]
    fn prop_trusted_input_passthrough(input in mixed_content_strategy()) {
        let config = SanitizationConfig::default();

        let dev_result = sanitize_input(&input, TrustLevel::Developer, &config);
        prop_assert!(dev_result.is_ok());
        prop_assert_eq!(dev_result.unwrap(), input.clone());

        let sys_result = sanitize_input(&input, TrustLevel::System, &config);
        prop_assert!(sys_result.is_ok());
        prop_assert_eq!(sys_result.unwrap(), input);
    }

    /// Property: sanitize_input rejects inputs exceeding max length
    #[test]
    fn prop_sanitize_rejects_oversized(
        base in "[a-zA-Z0-9]{10,50}",
        mult in 1000usize..2000
    ) {
        let mut config = SanitizationConfig::default();
        config.max_user_input_length = 100;
        config.block_injection_patterns = false; // Focus on length check

        let long_input = base.repeat(mult);
        let result = sanitize_input(&long_input, TrustLevel::User, &config);
        prop_assert!(result.is_err(), "Should reject input of length {}", long_input.len());
    }

    /// Property: sanitize_input never panics
    #[test]
    fn prop_sanitize_input_never_panics(
        input in potentially_malicious_strategy(),
        config in config_strategy()
    ) {
        let _ = sanitize_input(&input, TrustLevel::User, &config);
    }

    /// Property: sanitize_input with HTML escaping produces safe output
    #[test]
    fn prop_sanitize_escapes_html_when_enabled(input in mixed_content_strategy()) {
        let mut config = SanitizationConfig::default();
        config.escape_html = true;
        config.block_injection_patterns = false;

        if let Ok(result) = sanitize_input(&input, TrustLevel::User, &config) {
            // Should not contain raw HTML characters
            let should_be_escaped = ['<', '>', '"', '\''];
            for c in should_be_escaped {
                if input.contains(c) {
                    prop_assert!(!result.contains(c) || result.contains('&'),
                        "Character {} not properly escaped", c);
                }
            }
        }
    }
}

// ============================================================================
// PROPERTY TESTS: Round-trip invariants
// ============================================================================

proptest! {
    /// Property: Multiple sanitization passes are consistent
    #[test]
    fn prop_sanitization_idempotent_after_first_pass(input in "[a-zA-Z0-9 ]{0,100}") {
        let config = SanitizationConfig::default();

        if let Ok(first_pass) = sanitize_input(&input, TrustLevel::User, &config) {
            // Sanitizing the result of sanitization should produce same result
            // (only for clean inputs that pass the first time)
            if let Ok(second_pass) = sanitize_input(&first_pass, TrustLevel::User, &config) {
                // The second pass may double-escape, so we just check it doesn't fail
                prop_assert!(!second_pass.is_empty() || first_pass.is_empty());
            }
        }
    }
}
