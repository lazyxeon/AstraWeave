//! P2: Boundary Condition Tests for astraweave-prompts
//!
//! Focuses on edge cases for sanitization utilities:
//! - empty / very long inputs
//! - unicode and control characters
//! - strict length boundaries
//! - variable name validation boundaries

#![cfg(test)]
#![allow(clippy::field_reassign_with_default)]

use astraweave_prompts::sanitize::{
    sanitize_input, sanitize_variable_name, truncate_input, validate_safe_charset,
    SanitizationConfig, TrustLevel,
};

#[test]
fn test_sanitize_input_empty_string_ok() {
    let cfg = SanitizationConfig::default();
    let out = sanitize_input("", TrustLevel::User, &cfg).expect("empty input should be allowed");
    assert_eq!(out, "");
}

#[test]
fn test_sanitize_input_exact_max_length_ok() {
    let mut cfg = SanitizationConfig::default();
    cfg.max_user_input_length = 128;

    // Avoid triggering injection-pattern safeguards (e.g., 100+ consecutive identical chars).
    // Use a repeating, high-entropy base string and truncate to exactly max length.
    let base = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
    let repeat_count = cfg.max_user_input_length.div_ceil(base.len());
    let s: String = base
        .repeat(repeat_count)
        .chars()
        .take(cfg.max_user_input_length)
        .collect();
    let out = sanitize_input(&s, TrustLevel::User, &cfg).expect("exact max length should pass");
    assert_eq!(out.len(), cfg.max_user_input_length);
}

#[test]
fn test_sanitize_input_one_over_max_length_err() {
    let mut cfg = SanitizationConfig::default();
    cfg.max_user_input_length = 64;

    let input = "x".repeat(cfg.max_user_input_length + 1);
    let err = sanitize_input(&input, TrustLevel::User, &cfg).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("exceeds"), "unexpected error message: {msg}");
}

#[test]
fn test_sanitize_input_null_byte_blocked_by_default() {
    let cfg = SanitizationConfig::default();
    let input = "hello\0world";
    // Null bytes are now filtered out (not blocked) by default
    // Control characters are stripped when allow_control_chars is false
    let result = sanitize_input(input, TrustLevel::User, &cfg)
        .expect("null byte should be filtered, not blocked");
    assert!(!result.contains('\0'), "null byte should be removed");
    assert!(
        result.contains("helloworld"),
        "content should remain: {}",
        result
    );
}

#[test]
fn test_sanitize_input_null_byte_removed_when_pattern_blocking_disabled() {
    let mut cfg = SanitizationConfig::default();
    cfg.block_injection_patterns = false;
    cfg.allow_control_chars = false;

    let input = "hello\0world";
    let out = sanitize_input(input, TrustLevel::User, &cfg).expect("should sanitize without error");
    assert!(!out.contains('\0'), "control chars should be stripped");
    assert!(out.contains("helloworld"));
}

#[test]
fn test_sanitize_input_unicode_filtered_when_disallowed() {
    let mut cfg = SanitizationConfig::default();
    cfg.allow_unicode = false;
    cfg.block_injection_patterns = false;

    let input = "日本語🎮ABC";
    let out =
        sanitize_input(input, TrustLevel::User, &cfg).expect("unicode filtering should not error");
    assert!(out.contains("ABC"));
    assert!(out.is_ascii(), "output should be ASCII-only");
}

#[test]
fn test_sanitize_input_very_long_string_err_fast() {
    let mut cfg = SanitizationConfig::default();
    cfg.max_user_input_length = 10_000;

    // Intentionally oversized
    let input = "x".repeat(cfg.max_user_input_length + 1);
    assert!(sanitize_input(&input, TrustLevel::User, &cfg).is_err());
}

#[test]
fn test_sanitize_variable_name_empty_err() {
    let cfg = SanitizationConfig::default();
    assert!(sanitize_variable_name("", &cfg).is_err());
}

#[test]
fn test_sanitize_variable_name_max_length_ok() {
    let mut cfg = SanitizationConfig::default();
    cfg.max_variable_name_length = 32;

    let name = "a".repeat(cfg.max_variable_name_length);
    let out = sanitize_variable_name(&name, &cfg).expect("max length name should pass");
    assert_eq!(out.len(), cfg.max_variable_name_length);
}

#[test]
fn test_sanitize_variable_name_one_over_max_length_err() {
    let mut cfg = SanitizationConfig::default();
    cfg.max_variable_name_length = 8;

    let name = "a".repeat(cfg.max_variable_name_length + 1);
    assert!(sanitize_variable_name(&name, &cfg).is_err());
}

#[test]
fn test_sanitize_variable_name_must_start_with_letter_or_underscore() {
    let cfg = SanitizationConfig::default();
    assert!(sanitize_variable_name("1abc", &cfg).is_err());
    assert!(sanitize_variable_name("_abc", &cfg).is_ok());
}

#[test]
fn test_truncate_input_zero_max_length_is_empty() {
    let out = truncate_input("hello", 0);
    assert_eq!(out, "");
    assert!(out.len() <= 0);
}

#[test]
fn test_validate_safe_charset_null_byte_err() {
    let err = validate_safe_charset("hello\0world", true).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unsafe"), "unexpected error message: {msg}");
}
