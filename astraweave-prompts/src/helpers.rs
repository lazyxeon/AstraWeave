//! Helper utilities for prompt management
//!
//! This module provides various utility functions for prompt processing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::TemplateEngine;
use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};

/// Register default helpers to the template engine
pub fn register_default_helpers(engine: &mut TemplateEngine) {
    // JSON helper: serializes a variable to JSON
    engine.register_helper(
        "json",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "json", 0,
                        ))?;
                let json = serde_json::to_string_pretty(param.value())
                    .map_err(|e| handlebars::RenderErrorReason::Other(e.to_string()))?;
                out.write(&json)?;
                Ok(())
            },
        ),
    );

    // Trim helper: trims whitespace from the start and end of a string
    engine.register_helper(
        "trim",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "trim", 0,
                        ))?;
                let value = param
                    .value()
                    .as_str()
                    .ok_or(handlebars::RenderErrorReason::Other(
                        "Param must be a string".to_string(),
                    ))?;
                out.write(value.trim())?;
                Ok(())
            },
        ),
    );

    // Indent helper: indents text by a specified number of spaces (default 2)
    engine.register_helper(
        "indent",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "indent", 0,
                        ))?;
                let text = param
                    .value()
                    .as_str()
                    .ok_or(handlebars::RenderErrorReason::Other(
                        "Param must be a string".to_string(),
                    ))?;

                let spaces = if let Some(p1) = h.param(1) {
                    p1.value().as_u64().unwrap_or(2) as usize
                } else {
                    2
                };

                let indent_str = " ".repeat(spaces);
                let indented = text
                    .lines()
                    .map(|line| format!("{}{}", indent_str, line))
                    .collect::<Vec<_>>()
                    .join("\n");

                out.write(&indented)?;
                Ok(())
            },
        ),
    );

    // Uppercase helper
    engine.register_helper(
        "uppercase",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "uppercase",
                            0,
                        ))?;
                let value = param
                    .value()
                    .as_str()
                    .ok_or(handlebars::RenderErrorReason::Other(
                        "Param must be a string".to_string(),
                    ))?;
                out.write(&value.to_uppercase())?;
                Ok(())
            },
        ),
    );

    // Lowercase helper
    engine.register_helper(
        "lowercase",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "lowercase",
                            0,
                        ))?;
                let value = param
                    .value()
                    .as_str()
                    .ok_or(handlebars::RenderErrorReason::Other(
                        "Param must be a string".to_string(),
                    ))?;
                out.write(&value.to_lowercase())?;
                Ok(())
            },
        ),
    );

    // Length helper
    engine.register_helper(
        "length",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param =
                    h.param(0)
                        .ok_or(handlebars::RenderErrorReason::ParamNotFoundForIndex(
                            "length", 0,
                        ))?;
                let len = if let Some(s) = param.value().as_str() {
                    s.len()
                } else if let Some(arr) = param.value().as_array() {
                    arr.len()
                } else {
                    0
                };
                out.write(&len.to_string())?;
                Ok(())
            },
        ),
    );
}

/// Prompt validation utilities
pub struct PromptValidator;

/// Prompt formatting utilities
pub struct PromptFormatter;

/// Prompt analysis utilities
pub struct PromptAnalyzer;

/// Validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    /// Maximum prompt length
    pub max_length: usize,
    /// Minimum prompt length
    pub min_length: usize,
    /// Required variables
    pub required_variables: Vec<String>,
    /// Forbidden patterns
    pub forbidden_patterns: Vec<String>,
}

/// Formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattingOptions {
    /// Indent size for formatting
    pub indent_size: usize,
    /// Use tabs instead of spaces
    pub use_tabs: bool,
    /// Maximum line length
    pub max_line_length: usize,
}

/// Prompt analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Estimated token count
    pub estimated_tokens: usize,
    /// Complexity score (0-100)
    pub complexity_score: u8,
    /// Detected variables
    pub variables: Vec<String>,
    /// Readability score (0-100)
    pub readability_score: u8,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_length: 8000,
            min_length: 10,
            required_variables: Vec::new(),
            forbidden_patterns: Vec::new(),
        }
    }
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            indent_size: 2,
            use_tabs: false,
            max_line_length: 80,
        }
    }
}

impl PromptValidator {
    /// Validate a prompt against rules
    pub fn validate(prompt: &str, rules: &ValidationRules) -> Result<()> {
        // Length validation
        if prompt.len() > rules.max_length {
            anyhow::bail!("Prompt too long: {} > {}", prompt.len(), rules.max_length);
        }

        if prompt.len() < rules.min_length {
            anyhow::bail!("Prompt too short: {} < {}", prompt.len(), rules.min_length);
        }

        // Required variables validation
        for var in &rules.required_variables {
            let placeholder = format!("{{{}}}", var);
            if !prompt.contains(&placeholder) {
                anyhow::bail!("Required variable '{}' not found in prompt", var);
            }
        }

        // Forbidden patterns validation
        for pattern in &rules.forbidden_patterns {
            if prompt.contains(pattern) {
                anyhow::bail!("Forbidden pattern '{}' found in prompt", pattern);
            }
        }

        Ok(())
    }

    /// Check if prompt is well-formed
    pub fn is_well_formed(prompt: &str) -> bool {
        // Check for balanced braces
        let mut brace_count = 0;
        for char in prompt.chars() {
            match char {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                _ => {}
            }
            if brace_count < 0 {
                return false;
            }
        }

        brace_count == 0
    }
}

impl PromptFormatter {
    /// Format a prompt according to options
    pub fn format(prompt: &str, options: &FormattingOptions) -> String {
        let indent = if options.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(options.indent_size)
        };

        // Simple formatting - add indentation to lines
        prompt
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    line.to_string()
                } else if line.starts_with("  ") || line.starts_with("\t") {
                    format!("{}{}", indent, line.trim())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Minify a prompt (remove extra whitespace)
    pub fn minify(prompt: &str) -> String {
        prompt
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Pretty print a prompt with syntax highlighting placeholders
    pub fn pretty_print(prompt: &str) -> String {
        // Simple highlighting - wrap variables in markers
        let mut result = prompt.to_string();

        // Find variables and wrap them
        let variables = PromptAnalyzer::extract_variables(prompt);
        for var in variables {
            let placeholder = format!("{{{}}}", var);
            let highlighted = format!("**{{{}}}**", var);
            result = result.replace(&placeholder, &highlighted);
        }

        result
    }
}

impl PromptAnalyzer {
    /// Analyze a prompt and return analysis results
    pub fn analyze(prompt: &str) -> AnalysisResult {
        let estimated_tokens = Self::estimate_tokens(prompt);
        let complexity_score = Self::calculate_complexity(prompt);
        let variables = Self::extract_variables(prompt);
        let readability_score = Self::calculate_readability(prompt);

        AnalysisResult {
            estimated_tokens,
            complexity_score,
            variables,
            readability_score,
        }
    }

    /// Estimate token count (rough approximation)
    pub fn estimate_tokens(text: &str) -> usize {
        // Rough estimation: ~0.75 tokens per word
        let word_count = text.split_whitespace().count();
        ((word_count as f32) * 0.75) as usize
    }

    /// Calculate complexity score based on various factors
    pub fn calculate_complexity(prompt: &str) -> u8 {
        let mut score = 0u32;

        // Length factor (longer = more complex)
        score += (prompt.len() / 100).min(30) as u32;

        // Variable count factor
        let var_count = Self::extract_variables(prompt).len();
        score += (var_count * 5).min(25) as u32;

        // Nesting depth (based on brace levels)
        let mut max_depth = 0i32;
        let mut current_depth = 0i32;
        for char in prompt.chars() {
            match char {
                '{' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '}' => current_depth = current_depth.saturating_sub(1),
                _ => {}
            }
        }
        score += (max_depth * 3).min(15) as u32;

        // Line count factor
        let line_count = prompt.lines().count();
        score += (line_count / 10).min(30) as u32;

        score.min(100) as u8
    }

    /// Extract variables from prompt
    pub fn extract_variables(prompt: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut chars = prompt.chars().peekable();

        while let Some(char) = chars.next() {
            if char == '{' {
                let mut var_name = String::new();
                for char in chars.by_ref() {
                    if char == '}' {
                        break;
                    }
                    var_name.push(char);
                }

                if !var_name.is_empty() && !variables.contains(&var_name) {
                    variables.push(var_name);
                }
            }
        }

        variables
    }

    /// Calculate readability score
    pub fn calculate_readability(prompt: &str) -> u8 {
        let word_count = prompt.split_whitespace().count();
        let sentence_count = prompt.matches(['.', '!', '?']).count().max(1);
        let avg_words_per_sentence = word_count as f32 / sentence_count as f32;

        // Simple readability calculation
        let score = if avg_words_per_sentence <= 15.0 {
            90 - (avg_words_per_sentence * 2.0) as u8
        } else if avg_words_per_sentence <= 25.0 {
            70 - ((avg_words_per_sentence - 15.0) * 3.0) as u8
        } else {
            40 - ((avg_words_per_sentence - 25.0) * 1.5) as u8
        };

        score.min(100)
    }

    /// Generate prompt statistics
    pub fn generate_stats(prompt: &str) -> HashMap<String, String> {
        let mut stats = HashMap::new();

        stats.insert("character_count".to_string(), prompt.len().to_string());
        stats.insert(
            "word_count".to_string(),
            prompt.split_whitespace().count().to_string(),
        );
        stats.insert("line_count".to_string(), prompt.lines().count().to_string());
        stats.insert(
            "variable_count".to_string(),
            Self::extract_variables(prompt).len().to_string(),
        );
        stats.insert(
            "estimated_tokens".to_string(),
            Self::estimate_tokens(prompt).to_string(),
        );

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PromptValidator tests
    #[test]
    fn test_validate_valid_prompt() {
        let prompt = "This is a valid prompt with {variable}.";
        let mut rules = ValidationRules::default();
        rules.required_variables.push("variable".to_string());

        assert!(PromptValidator::validate(prompt, &rules).is_ok());
    }

    #[test]
    fn test_validate_prompt_too_long() {
        let prompt = "a".repeat(10000);
        let rules = ValidationRules {
            max_length: 1000,
            ..Default::default()
        };

        let result = PromptValidator::validate(&prompt, &rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_validate_prompt_too_short() {
        let prompt = "Hi";
        let rules = ValidationRules {
            min_length: 10,
            ..Default::default()
        };

        let result = PromptValidator::validate(prompt, &rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_validate_missing_required_variable() {
        let prompt = "This prompt has no variables.";
        let rules = ValidationRules {
            required_variables: vec!["name".to_string()],
            ..Default::default()
        };

        let result = PromptValidator::validate(prompt, &rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn test_validate_forbidden_pattern() {
        let prompt = "This prompt contains FORBIDDEN text.";
        let rules = ValidationRules {
            forbidden_patterns: vec!["FORBIDDEN".to_string()],
            ..Default::default()
        };

        let result = PromptValidator::validate(prompt, &rules);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("FORBIDDEN"));
    }

    #[test]
    fn test_is_well_formed_balanced() {
        assert!(PromptValidator::is_well_formed("Hello {world}!"));
        assert!(PromptValidator::is_well_formed("{a}{b}{c}"));
        assert!(PromptValidator::is_well_formed("No braces here"));
    }

    #[test]
    fn test_is_well_formed_unbalanced() {
        assert!(!PromptValidator::is_well_formed("Hello {world"));
        assert!(!PromptValidator::is_well_formed("Hello world}"));
        assert!(!PromptValidator::is_well_formed("{{}"));
    }

    // PromptFormatter tests
    #[test]
    fn test_format_with_spaces() {
        let prompt = "Line 1\n  Indented line\nLine 3";
        let options = FormattingOptions::default();
        let formatted = PromptFormatter::format(prompt, &options);

        assert!(formatted.contains("Line 1"));
        assert!(formatted.contains("Line 3"));
    }

    #[test]
    fn test_format_with_tabs() {
        let prompt = "Line 1\n  Indented";
        let options = FormattingOptions {
            use_tabs: true,
            ..Default::default()
        };
        let formatted = PromptFormatter::format(prompt, &options);

        assert!(formatted.contains("Line 1"));
    }

    #[test]
    fn test_minify() {
        let prompt = "Line 1\n  \n  Line 2  \n\nLine 3";
        let minified = PromptFormatter::minify(prompt);

        assert_eq!(minified, "Line 1 Line 2 Line 3");
    }

    #[test]
    fn test_pretty_print() {
        let prompt = "Hello {name}, welcome to {place}!";
        let pretty = PromptFormatter::pretty_print(prompt);

        assert!(pretty.contains("**{name}**"));
        assert!(pretty.contains("**{place}**"));
    }

    // PromptAnalyzer tests
    #[test]
    fn test_estimate_tokens() {
        let text = "This is a test sentence with eight words here.";
        let tokens = PromptAnalyzer::estimate_tokens(text);

        // 10 words * 0.75 ≈ 7
        assert!(tokens > 0);
        assert!(tokens < text.split_whitespace().count());
    }

    #[test]
    fn test_extract_variables() {
        let prompt = "Hello {name}! Your {item} is ready at {location}.";
        let vars = PromptAnalyzer::extract_variables(prompt);

        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"item".to_string()));
        assert!(vars.contains(&"location".to_string()));
    }

    #[test]
    fn test_extract_variables_no_duplicates() {
        let prompt = "{name} likes {name}";
        let vars = PromptAnalyzer::extract_variables(prompt);

        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn test_extract_variables_empty() {
        let prompt = "No variables here";
        let vars = PromptAnalyzer::extract_variables(prompt);

        assert!(vars.is_empty());
    }

    #[test]
    fn test_calculate_complexity_short() {
        let prompt = "Simple prompt.";
        let score = PromptAnalyzer::calculate_complexity(prompt);

        assert!(score < 50);
    }

    #[test]
    fn test_calculate_complexity_long() {
        let prompt = "a ".repeat(500) + &"{var}".repeat(10);
        let score = PromptAnalyzer::calculate_complexity(&prompt);

        // Long prompts should have some complexity score
        assert!(score > 0);
    }

    #[test]
    fn test_calculate_readability() {
        let prompt = "Short. Sentences. Are. Easy.";
        let score = PromptAnalyzer::calculate_readability(prompt);

        // Short sentences = high readability
        assert!(score > 50);
    }

    #[test]
    fn test_analyze() {
        let prompt = "Hello {name}! Welcome to {place}. This is a test.";
        let result = PromptAnalyzer::analyze(prompt);

        assert!(result.estimated_tokens > 0);
        assert!(result.complexity_score <= 100);
        assert_eq!(result.variables.len(), 2);
        assert!(result.readability_score <= 100);
    }

    #[test]
    fn test_generate_stats() {
        let prompt = "Hello {name}!\nLine two.";
        let stats = PromptAnalyzer::generate_stats(prompt);

        assert!(stats.contains_key("character_count"));
        assert!(stats.contains_key("word_count"));
        assert!(stats.contains_key("line_count"));
        assert!(stats.contains_key("variable_count"));
        assert!(stats.contains_key("estimated_tokens"));

        assert_eq!(stats.get("line_count").unwrap(), "2");
        assert_eq!(stats.get("variable_count").unwrap(), "1");
    }

    // ValidationRules tests
    #[test]
    fn test_validation_rules_default() {
        let rules = ValidationRules::default();

        assert_eq!(rules.max_length, 8000);
        assert_eq!(rules.min_length, 10);
        assert!(rules.required_variables.is_empty());
        assert!(rules.forbidden_patterns.is_empty());
    }

    #[test]
    fn test_validation_rules_serialization() {
        let rules = ValidationRules {
            max_length: 5000,
            min_length: 50,
            required_variables: vec!["name".to_string()],
            forbidden_patterns: vec!["bad".to_string()],
        };

        let serialized = serde_json::to_string(&rules).unwrap();
        let deserialized: ValidationRules = serde_json::from_str(&serialized).unwrap();

        assert_eq!(rules.max_length, deserialized.max_length);
        assert_eq!(rules.required_variables, deserialized.required_variables);
    }

    // FormattingOptions tests
    #[test]
    fn test_formatting_options_default() {
        let options = FormattingOptions::default();

        assert_eq!(options.indent_size, 2);
        assert!(!options.use_tabs);
        assert_eq!(options.max_line_length, 80);
    }

    #[test]
    fn test_formatting_options_serialization() {
        let options = FormattingOptions {
            indent_size: 4,
            use_tabs: true,
            max_line_length: 120,
        };

        let serialized = serde_json::to_string(&options).unwrap();
        let deserialized: FormattingOptions = serde_json::from_str(&serialized).unwrap();

        assert_eq!(options.indent_size, deserialized.indent_size);
        assert_eq!(options.use_tabs, deserialized.use_tabs);
    }

    // AnalysisResult tests
    #[test]
    fn test_analysis_result_serialization() {
        let result = AnalysisResult {
            estimated_tokens: 100,
            complexity_score: 50,
            variables: vec!["name".to_string()],
            readability_score: 75,
        };

        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: AnalysisResult = serde_json::from_str(&serialized).unwrap();

        assert_eq!(result.estimated_tokens, deserialized.estimated_tokens);
        assert_eq!(result.variables, deserialized.variables);
    }

    // Edge cases
    #[test]
    fn test_empty_prompt_analyze() {
        let prompt = "";
        let result = PromptAnalyzer::analyze(prompt);

        assert_eq!(result.estimated_tokens, 0);
        assert!(result.variables.is_empty());
    }

    #[test]
    fn test_nested_braces() {
        let prompt = "{{nested}}";
        let vars = PromptAnalyzer::extract_variables(prompt);

        // Should handle nested braces by extracting inner variable
        assert!(!vars.is_empty());
    }

    #[test]
    fn test_minify_empty_lines() {
        let prompt = "\n\n\n";
        let minified = PromptFormatter::minify(prompt);

        assert!(minified.is_empty());
    }

    #[test]
    fn test_format_empty_lines_preserved() {
        let prompt = "Line 1\n\nLine 3";
        let options = FormattingOptions::default();
        let formatted = PromptFormatter::format(prompt, &options);

        assert!(formatted.contains("\n"));
    }

    // === Mutation Remediation Tests ===

    #[test]
    fn test_validate_at_exact_max_length_passes() {
        // Remediation: kills "replace > with >= in PromptValidator::validate" (line 261)
        let prompt = "a".repeat(100);
        let rules = ValidationRules {
            max_length: 100,
            min_length: 0,
            ..Default::default()
        };
        assert!(
            PromptValidator::validate(&prompt, &rules).is_ok(),
            "prompt at exact max_length should pass"
        );
    }

    #[test]
    fn test_validate_at_exact_min_length_passes() {
        // Remediation: kills "replace < with <= in PromptValidator::validate" (line 265)
        let prompt = "a".repeat(10);
        let rules = ValidationRules {
            max_length: 8000,
            min_length: 10,
            ..Default::default()
        };
        assert!(
            PromptValidator::validate(&prompt, &rules).is_ok(),
            "prompt at exact min_length should pass"
        );
    }

    #[test]
    fn test_calculate_complexity_nesting_depth_factor() {
        // Remediation: kills match arm '{'/'}' deletion and += mutations in calculate_complexity
        let no_braces = "simple prompt with no braces";
        let nested = "outer { inner { deep } end }";
        let score_flat = PromptAnalyzer::calculate_complexity(no_braces);
        let score_nested = PromptAnalyzer::calculate_complexity(nested);
        assert!(
            score_nested > score_flat,
            "nested braces should increase complexity: nested={} flat={}",
            score_nested,
            score_flat
        );
    }

    #[test]
    fn test_calculate_complexity_length_factor() {
        // Remediation: kills "replace += with *= / *→/" in len/100 factor
        let short = "x";
        let long = "x".repeat(500);
        let score_short = PromptAnalyzer::calculate_complexity(short);
        let score_long = PromptAnalyzer::calculate_complexity(&long);
        assert!(
            score_long > score_short,
            "longer prompts should have higher complexity: long={} short={}",
            score_long,
            score_short
        );
    }

    #[test]
    fn test_calculate_complexity_line_count_factor() {
        // Remediation: kills "replace += with -= / *→/" in line_count factor
        let one_line = "a single line prompt";
        let many_lines = (0..50)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let score_one = PromptAnalyzer::calculate_complexity(one_line);
        let score_many = PromptAnalyzer::calculate_complexity(&many_lines);
        assert!(
            score_many > score_one,
            "more lines should increase complexity: many={} one={}",
            score_many,
            score_one
        );
    }

    #[test]
    fn test_calculate_readability_short_sentences_high_score() {
        // Remediation: kills arithmetic mutations in calculate_readability
        // Short sentences (<=15 words/sentence) should give high readability
        let short_sentences = "Hello world. This is nice. Good day.";
        let score = PromptAnalyzer::calculate_readability(short_sentences);
        // ~3 sentences, ~7 words → avg ~2.3 words/sentence → score = 90 - 4 = 86
        assert!(
            score > 60,
            "short sentences should have high readability, got {}",
            score
        );
    }

    #[test]
    fn test_calculate_readability_long_sentences_lower_score() {
        // Remediation: kills branch and arithmetic mutations in middle/long branches
        let long_sentence = (0..25).map(|_| "word").collect::<Vec<_>>().join(" ") + ".";
        let score_long = PromptAnalyzer::calculate_readability(&long_sentence);
        let short_sentence = "Short sentence.";
        let score_short = PromptAnalyzer::calculate_readability(short_sentence);
        assert!(
            score_short > score_long,
            "short sentences should score higher than long: short={} long={}",
            score_short,
            score_long
        );
    }

    #[test]
    fn test_calculate_readability_very_long_uses_third_branch() {
        // Remediation: kills >= 25 branch arithmetic mutations
        let very_long = (0..40).map(|_| "word").collect::<Vec<_>>().join(" ") + ".";
        let score = PromptAnalyzer::calculate_readability(&very_long);
        // 40 words/1 sentence → 40 avg → score = 40 - ((40-25)*1.5) = 40-22 = 18
        assert!(
            score < 50,
            "very long sentences should score low, got {}",
            score
        );
        assert!(score > 0, "score should be positive, got {}", score);
    }

    #[test]
    fn test_calculate_complexity_exact_score_kills_arithmetic_and_branch_mutants() {
        // Remediation: kills 6 MISSED mutants in calculate_complexity:
        //   - helpers.rs:390  var_count * 5 → / 5
        //   - helpers.rs:397  delete '{' match arm
        //   - helpers.rs:398  current_depth += 1 → -= 1 / *= 1
        //   - helpers.rs:401  delete '}' match arm
        //   - helpers.rs:405  max_depth * 3 → / 3
        //
        // Prompt "{a}{b}" (len=6):
        //   length factor:  6/100 = 0
        //   var_count = 2:  2*5 = 10
        //   nesting depth:  max_depth=1 → 1*3 = 3
        //   line count:     1/10 = 0
        //   total = 13
        let score = PromptAnalyzer::calculate_complexity("{a}{b}");
        assert_eq!(
            score, 13,
            "complexity of '{{a}}{{b}}' must be exactly 13 (var=10 + nest=3)"
        );

        // Second assertion: a prompt with NO braces has zero nesting contribution.
        // This pins the nesting factor independently.
        let flat_score = PromptAnalyzer::calculate_complexity("hello world");
        assert_eq!(flat_score, 0, "short flat prompt must have complexity 0");
    }

    #[test]
    fn test_calculate_readability_exact_scores_kills_arithmetic_mutants() {
        // Remediation: kills 10 MISSED mutants in calculate_readability
        // lines 446, 448, 450 — all arithmetic ops (*, -, /) in 3 branches

        // Branch 1 (avg <= 15.0): 10 words, 1 sentence
        // avg = 10.0 -> score = 90 - (10.0 * 2.0) as u8 = 70
        let b1 = (0..10)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ")
            + ".";
        assert_eq!(
            PromptAnalyzer::calculate_readability(&b1),
            70,
            "branch1: 10 words/1 sentence -> 70"
        );

        // Branch 2 (15 < avg <= 25): 20 words, 1 sentence
        // avg = 20.0 -> score = 70 - ((20.0 - 15.0) * 3.0) as u8 = 55
        let b2 = (0..20)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ")
            + ".";
        assert_eq!(
            PromptAnalyzer::calculate_readability(&b2),
            55,
            "branch2: 20 words/1 sentence -> 55"
        );

        // Branch 3 (avg > 25): 40 words, 1 sentence
        // avg = 40.0 -> score = 40 - ((40.0 - 25.0) * 1.5) as u8 = 40 - 22 = 18
        let b3 = (0..40)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ")
            + ".";
        assert_eq!(
            PromptAnalyzer::calculate_readability(&b3),
            18,
            "branch3: 40 words/1 sentence -> 18"
        );
    }
}
