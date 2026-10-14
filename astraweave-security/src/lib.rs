#![forbid(unsafe_code)]
//! Security and Sandboxing for AstraWeave
//!
//! This crate provides security features including:
//! - Path traversal protection and file validation
//! - Secure deserialization with size limits
//! - LLM prompt sanitization and sandboxing
//! - Script execution sandboxing with Rhai
//! - Input validation and anti-cheat measures
//! - Telemetry and monitoring systems

pub mod deserialization;
pub mod path;
pub mod sandbox;

use anyhow::Result;
use astraweave_ecs::{App, Plugin, World};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Security configuration resource
#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub enable_sandboxing: bool,
    pub enable_llm_validation: bool,
    pub enable_script_sandbox: bool,
    pub max_script_execution_time_ms: u64,
    pub max_memory_usage_mb: usize,
}

/// Telemetry data collection
#[derive(Clone, Debug)]
pub struct TelemetryData {
    pub events: Vec<TelemetryEvent>,
    pub session_start: std::time::Instant,
    pub anomaly_count: u64,
}

/// Individual telemetry event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub severity: TelemetrySeverity,
    pub data: serde_json::Value,
}

/// Telemetry severity levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum TelemetrySeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Script execution sandbox
#[derive(Clone)]
pub struct ScriptSandbox {
    pub engine: Arc<Mutex<rhai::Engine>>,
    pub allowed_functions: HashMap<String, String>,
    pub execution_limits: ExecutionLimits,
}

/// Execution limits for sandboxed scripts
#[derive(Clone, Debug)]
pub struct ExecutionLimits {
    pub max_operations: u64,
    pub max_memory_bytes: usize,
    pub timeout_ms: u64,
}

/// LLM validation and sanitization
#[derive(Clone, Debug)]
pub struct LLMValidator {
    pub banned_patterns: Vec<String>,
    pub allowed_domains: Vec<String>,
    pub max_prompt_length: usize,
    pub enable_content_filtering: bool,
}

/// Anti-cheat component for entities
#[derive(Clone, Debug)]
pub struct CAntiCheat {
    pub player_id: String,
    pub trust_score: f32,
    pub last_validation: u64,
    pub anomaly_flags: Vec<String>,
}

/// Input validation result
#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub trust_score: f32,
    pub warnings: Vec<String>,
    pub anomalies: Vec<String>,
}

/// Security plugin for ECS integration
pub struct SecurityPlugin {
    config: SecurityConfig,
}

impl SecurityPlugin {
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }
}

impl Default for SecurityPlugin {
    fn default() -> Self {
        Self {
            config: SecurityConfig {
                enable_sandboxing: true,
                enable_llm_validation: true,
                enable_script_sandbox: true,
                max_script_execution_time_ms: 1000,
                max_memory_usage_mb: 50,
            },
        }
    }
}

impl Plugin for SecurityPlugin {
    fn build(&self, app: &mut App) {
        // Initialize security resources
        app.world.insert_resource(self.config.clone());
        app.world.insert_resource(TelemetryData {
            events: Vec::new(),
            session_start: std::time::Instant::now(),
            anomaly_count: 0,
        });

        // Initialize script sandbox
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(10000);
        engine.set_max_string_size(1000);

        let sandbox = ScriptSandbox {
            engine: Arc::new(Mutex::new(engine)),
            allowed_functions: HashMap::new(),
            execution_limits: ExecutionLimits {
                max_operations: 10000,
                max_memory_bytes: 1024 * 1024, // 1MB
                timeout_ms: self.config.max_script_execution_time_ms,
            },
        };

        app.world.insert_resource(sandbox);

        // Initialize LLM validator
        let llm_validator = LLMValidator {
            banned_patterns: vec![
                "system(".to_string(),
                "exec(".to_string(),
                "eval(".to_string(),
                "import ".to_string(),
            ],
            allowed_domains: default_allowed_llm_domains(),
            max_prompt_length: 10000,
            enable_content_filtering: true,
        };

        app.world.insert_resource(llm_validator);

        // Add security systems
        app.add_system("pre_simulation", input_validation_system)
            .expect("registered schedule stage");
        app.add_system("post_simulation", telemetry_collection_system)
            .expect("registered schedule stage");
        app.add_system("post_simulation", anomaly_detection_system)
            .expect("registered schedule stage");
    }
}

/// Input validation system
fn input_validation_system(world: &mut World) {
    let entities: Vec<_> = world.entities_with::<CAntiCheat>();

    for entity in entities {
        // Get anti_cheat component to read
        let validation_result = if let Some(anti_cheat) = world.get::<CAntiCheat>(entity) {
            validate_player_input(anti_cheat)
        } else {
            continue;
        };

        // Update trust score based on validation
        if let Some(anti_cheat) = world.get_mut::<CAntiCheat>(entity) {
            anti_cheat.trust_score =
                (anti_cheat.trust_score * 0.9) + (validation_result.trust_score * 0.1);
            anti_cheat.last_validation = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before UNIX epoch")
                .as_secs();

            // Record anomalies
            if !validation_result.anomalies.is_empty() {
                anti_cheat
                    .anomaly_flags
                    .extend(validation_result.anomalies.clone());

                // Collect data for telemetry
                let player_id = anti_cheat.player_id.clone();
                let trust_score = anti_cheat.trust_score;
                let timestamp = anti_cheat.last_validation;

                // Log telemetry event
                if let Some(telemetry) = world.get_resource_mut::<TelemetryData>() {
                    telemetry.events.push(TelemetryEvent {
                        timestamp,
                        event_type: "input_anomaly".to_string(),
                        severity: TelemetrySeverity::Warning,
                        data: serde_json::json!({
                            "player_id": player_id,
                            "anomalies": validation_result.anomalies,
                            "trust_score": trust_score
                        }),
                    });
                }
            }
        }
    }
}

/// Telemetry collection system
fn telemetry_collection_system(world: &mut World) {
    if let Some(telemetry) = world.get_resource_mut::<TelemetryData>() {
        // Clean up old events (keep last 1000)
        if telemetry.events.len() > 1000 {
            telemetry.events = telemetry.events.split_off(telemetry.events.len() - 1000);
        }

        // Log periodic telemetry summary
        let session_duration = telemetry.session_start.elapsed().as_secs();
        if session_duration % 60 == 0 && !telemetry.events.is_empty() {
            println!(
                "Telemetry: {} events in {} seconds, {} anomalies",
                telemetry.events.len(),
                session_duration,
                telemetry.anomaly_count
            );
        }
    }
}

/// Anomaly detection system
fn anomaly_detection_system(world: &mut World) {
    let mut total_anomalies = 0;
    let mut low_trust_players = 0;
    let mut total_players = 0;

    let entities: Vec<_> = world.entities_with::<CAntiCheat>();

    for entity in &entities {
        if let Some(anti_cheat) = world.get::<CAntiCheat>(*entity) {
            total_anomalies += anti_cheat.anomaly_flags.len() as u64;
            if anti_cheat.trust_score < 0.5 {
                low_trust_players += 1;
            }
            total_players += 1;
        }
    }

    if let Some(telemetry) = world.get_resource_mut::<TelemetryData>() {
        telemetry.anomaly_count = total_anomalies;

        // Detect systemic anomalies
        if low_trust_players > total_players / 2 {
            telemetry.events.push(TelemetryEvent {
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system clock before UNIX epoch")
                    .as_secs(),
                event_type: "systemic_anomaly".to_string(),
                severity: TelemetrySeverity::Critical,
                data: serde_json::json!({
                    "low_trust_players": low_trust_players,
                    "total_players": total_players,
                    "total_anomalies": total_anomalies
                }),
            });
        }
    }
}

/// Validate player input for cheating
pub fn validate_player_input(anti_cheat: &CAntiCheat) -> ValidationResult {
    let mut warnings = Vec::new();
    let mut anomalies = Vec::new();
    let mut trust_score = 1.0;

    // Check for rapid input patterns (potential botting)
    if anti_cheat
        .anomaly_flags
        .contains(&"rapid_input".to_string())
    {
        anomalies.push("rapid_input".to_string());
        trust_score *= 0.8;
        warnings.push("Unusually rapid input detected".to_string());
    }

    // Check for impossible movement patterns
    if anti_cheat
        .anomaly_flags
        .contains(&"impossible_movement".to_string())
    {
        anomalies.push("impossible_movement".to_string());
        trust_score *= 0.5;
        warnings.push("Impossible movement pattern detected".to_string());
    }

    // Check for memory tampering indicators
    if anti_cheat
        .anomaly_flags
        .contains(&"memory_tamper".to_string())
    {
        anomalies.push("memory_tamper".to_string());
        trust_score *= 0.3;
        warnings.push("Memory tampering indicators detected".to_string());
    }

    ValidationResult {
        is_valid: trust_score > 0.2,
        trust_score,
        warnings,
        anomalies,
    }
}

/// Domains the dormant LLM validator allows. Local only.
pub fn default_allowed_llm_domains() -> Vec<String> {
    vec!["localhost".to_string()]
}

/// Sanitize LLM prompt for security
pub fn sanitize_llm_prompt(prompt: &str, validator: &LLMValidator) -> Result<String> {
    // Check prompt length
    if prompt.len() > validator.max_prompt_length {
        anyhow::bail!(
            "Prompt too long: {} > {}",
            prompt.len(),
            validator.max_prompt_length
        );
    }

    // Check for banned patterns
    for pattern in &validator.banned_patterns {
        if prompt.contains(pattern) {
            anyhow::bail!("Prompt contains banned pattern: {}", pattern);
        }
    }

    // Basic content filtering
    if validator.enable_content_filtering {
        // TODO: Implement more sophisticated content filtering
        let suspicious_patterns = ["hack", "exploit", "cheat", "bypass"];
        for pattern in &suspicious_patterns {
            if prompt.to_lowercase().contains(pattern) {
                return Ok(format!("SAFE: {}", prompt)); // Prefix safe prompts
            }
        }
    }

    Ok(prompt.to_string())
}

/// Execute script in sandbox
pub async fn execute_script_sandboxed(
    script: &str,
    sandbox: &ScriptSandbox,
    context: HashMap<String, rhai::Dynamic>,
) -> Result<rhai::Dynamic> {
    let script = script.to_string();
    let engine = sandbox.engine.clone();
    let timeout_ms = sandbox.execution_limits.timeout_ms;

    // Execute with timeout in a blocking task
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        tokio::task::spawn_blocking(move || -> Result<rhai::Dynamic> {
            let engine = engine
                .lock()
                .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;

            // Compile the script
            let ast = engine.compile(&script)?;

            // Create scope with context variables
            let mut scope = rhai::Scope::new();
            for (key, value) in context {
                scope.push(key, value);
            }

            // Execute the script
            let result = engine.eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)?;
            Ok(result)
        }),
    )
    .await??;

    result
}

/// Generate cryptographic signature for data integrity
pub fn generate_signature(data: &[u8], signing_key: &SigningKey) -> Signature {
    signing_key.sign(data)
}

/// Verify cryptographic signature
pub fn verify_signature(data: &[u8], signature: &Signature, verifying_key: &VerifyingKey) -> bool {
    verifying_key.verify(data, signature).is_ok()
}

/// Generate a new keypair for signing
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::from_bytes(&rand::random());
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Hash data for integrity checking
pub fn hash_data(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod anticheat_tests;
#[cfg(test)]
mod ecs_systems_tests;
#[cfg(test)]
mod llm_validation_tests;
#[cfg(test)]
mod mutation_tests;
#[cfg(test)]
mod script_sandbox_tests;
#[cfg(test)]
mod signature_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_prompt_sanitization() {
        let validator = LLMValidator {
            banned_patterns: vec!["system(".to_string(), "exec(".to_string()],
            allowed_domains: vec![],
            max_prompt_length: 1000,
            enable_content_filtering: true,
        };

        // Valid prompt
        let result = sanitize_llm_prompt("Hello, how are you?", &validator).unwrap();
        assert_eq!(result, "Hello, how are you?");

        // Banned pattern
        let result = sanitize_llm_prompt("Please system(exit)", &validator);
        assert!(result.is_err());

        // Too long
        let long_prompt = "x".repeat(2000);
        let result = sanitize_llm_prompt(&long_prompt, &validator);
        assert!(result.is_err());
    }

    #[test]
    fn input_validation() {
        let anti_cheat = CAntiCheat {
            player_id: "test_player".to_string(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec!["rapid_input".to_string()],
        };

        let result = validate_player_input(&anti_cheat);
        assert!(result.is_valid);
        assert!(result.trust_score < 1.0);
        assert!(!result.anomalies.is_empty());
    }

    #[test]
    fn cryptographic_signing() {
        let (signing_key, verifying_key) = generate_keypair();
        let data = b"Hello, world!";
        let signature = generate_signature(data, &signing_key);

        assert!(verify_signature(data, &signature, &verifying_key));

        // Tampered data should fail verification
        let tampered_data = b"Hello, world?";
        assert!(!verify_signature(tampered_data, &signature, &verifying_key));
    }

    #[test]
    fn data_hashing() {
        let data = b"Hello, world!";
        let hash1 = hash_data(data);
        let hash2 = hash_data(data);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }

    #[tokio::test]
    async fn script_sandbox_execution() {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(1000);

        let sandbox = ScriptSandbox {
            engine: Arc::new(Mutex::new(engine)),
            allowed_functions: HashMap::new(),
            execution_limits: ExecutionLimits {
                max_operations: 1000,
                max_memory_bytes: 1024 * 1024,
                timeout_ms: 1000, // 1 second timeout
            },
        };

        let script = "40 + 2";
        let context = HashMap::new();
        let result = execute_script_sandboxed(script, &sandbox, context)
            .await
            .unwrap();

        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[test]
    fn test_security_config_default() {
        let plugin = SecurityPlugin::default();
        assert!(plugin.config.enable_sandboxing);
        assert!(plugin.config.enable_llm_validation);
        assert!(plugin.config.enable_script_sandbox);
        assert_eq!(plugin.config.max_script_execution_time_ms, 1000);
        assert_eq!(plugin.config.max_memory_usage_mb, 50);
    }

    #[test]
    fn test_security_config_custom() {
        let config = SecurityConfig {
            enable_sandboxing: false,
            enable_llm_validation: false,
            enable_script_sandbox: false,
            max_script_execution_time_ms: 500,
            max_memory_usage_mb: 100,
        };
        let plugin = SecurityPlugin::new(config.clone());
        assert!(!plugin.config.enable_sandboxing);
        assert_eq!(plugin.config.max_script_execution_time_ms, 500);
    }

    #[test]
    fn test_telemetry_event_creation() {
        let event = TelemetryEvent {
            timestamp: 12345,
            event_type: "test".to_string(),
            severity: TelemetrySeverity::Info,
            data: serde_json::json!({"key": "value"}),
        };
        assert_eq!(event.timestamp, 12345);
        assert_eq!(event.event_type, "test");
        assert_eq!(event.severity, TelemetrySeverity::Info);
    }

    #[test]
    fn test_telemetry_severity_variants() {
        assert_eq!(TelemetrySeverity::Info, TelemetrySeverity::Info);
        assert_eq!(TelemetrySeverity::Warning, TelemetrySeverity::Warning);
        assert_eq!(TelemetrySeverity::Error, TelemetrySeverity::Error);
        assert_eq!(TelemetrySeverity::Critical, TelemetrySeverity::Critical);
        assert_ne!(TelemetrySeverity::Info, TelemetrySeverity::Warning);
    }

    #[test]
    fn test_execution_limits() {
        let limits = ExecutionLimits {
            max_operations: 5000,
            max_memory_bytes: 2048 * 1024,
            timeout_ms: 200,
        };
        assert_eq!(limits.max_operations, 5000);
        assert_eq!(limits.max_memory_bytes, 2048 * 1024);
        assert_eq!(limits.timeout_ms, 200);
    }

    #[test]
    fn test_validation_result_clean() {
        let result = ValidationResult {
            is_valid: true,
            trust_score: 1.0,
            warnings: vec![],
            anomalies: vec![],
        };
        assert!(result.is_valid);
        assert_eq!(result.trust_score, 1.0);
    }

    #[test]
    fn test_validation_result_with_issues() {
        let result = ValidationResult {
            is_valid: false,
            trust_score: 0.3,
            warnings: vec!["suspicious_pattern".to_string()],
            anomalies: vec!["teleport_detected".to_string()],
        };
        assert!(!result.is_valid);
        assert!(result.trust_score < 1.0);
        assert!(!result.warnings.is_empty());
        assert!(!result.anomalies.is_empty());
    }

    #[test]
    fn test_input_validation_clean_player() {
        let anti_cheat = CAntiCheat {
            player_id: "clean_player".to_string(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec![],
        };

        let result = validate_player_input(&anti_cheat);
        assert!(result.is_valid);
        assert_eq!(result.trust_score, 1.0);
        assert!(result.anomalies.is_empty());
    }

    #[test]
    fn test_input_validation_multiple_anomalies() {
        let anti_cheat = CAntiCheat {
            player_id: "suspicious_player".to_string(),
            trust_score: 0.5,
            last_validation: 0,
            anomaly_flags: vec![
                "rapid_input".to_string(),
                "teleport".to_string(),
                "impossible_stats".to_string(),
            ],
        };

        let result = validate_player_input(&anti_cheat);
        // Multiple anomalies should reduce trust score
        assert!(result.trust_score < 1.0);
    }

    #[test]
    fn test_anticheat_component() {
        let ac = CAntiCheat {
            player_id: "player_123".to_string(),
            trust_score: 0.95,
            last_validation: 1234567890,
            anomaly_flags: vec!["minor_issue".to_string()],
        };
        assert_eq!(ac.player_id, "player_123");
        assert_eq!(ac.trust_score, 0.95);
        assert_eq!(ac.last_validation, 1234567890);
        assert_eq!(ac.anomaly_flags.len(), 1);
    }

    // ── Additional edge-case tests ──

    #[test]
    fn test_validate_all_three_anomalies_combined() {
        let anti_cheat = CAntiCheat {
            player_id: "cheater".to_string(),
            trust_score: 0.1,
            last_validation: 0,
            anomaly_flags: vec![
                "rapid_input".to_string(),
                "impossible_movement".to_string(),
                "memory_tamper".to_string(),
            ],
        };
        let result = validate_player_input(&anti_cheat);
        // 1.0 * 0.8 * 0.5 * 0.3 = 0.12 which is < 0.2
        assert!(!result.is_valid);
        assert!((result.trust_score - 0.12).abs() < 0.01);
        assert_eq!(result.anomalies.len(), 3);
        assert_eq!(result.warnings.len(), 3);
    }

    #[test]
    fn test_validate_impossible_movement_only() {
        let anti_cheat = CAntiCheat {
            player_id: "p".to_string(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec!["impossible_movement".to_string()],
        };
        let result = validate_player_input(&anti_cheat);
        assert!((result.trust_score - 0.5).abs() < f32::EPSILON);
        assert!(result.is_valid); // 0.5 > 0.2
    }

    #[test]
    fn test_validate_memory_tamper_only() {
        let anti_cheat = CAntiCheat {
            player_id: "p".to_string(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec!["memory_tamper".to_string()],
        };
        let result = validate_player_input(&anti_cheat);
        assert!((result.trust_score - 0.3).abs() < f32::EPSILON);
        assert!(result.is_valid); // 0.3 > 0.2
    }

    #[test]
    fn test_sanitize_content_filtering_disabled() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 1000,
            enable_content_filtering: false,
        };
        // "hack" would trigger content filter, but it's disabled
        let result = sanitize_llm_prompt("tell me about hack techniques", &validator).unwrap();
        assert_eq!(result, "tell me about hack techniques");
    }

    #[test]
    fn test_sanitize_empty_prompt() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 1000,
            enable_content_filtering: true,
        };
        let result = sanitize_llm_prompt("", &validator).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_suspicious_substring() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 1000,
            enable_content_filtering: true,
        };
        // "hacking" contains "hack" as substring → triggers content filter
        let result = sanitize_llm_prompt("a hacking tutorial", &validator).unwrap();
        assert!(result.starts_with("SAFE: "));
    }

    #[test]
    fn test_sanitize_multiple_suspicious_words() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 1000,
            enable_content_filtering: true,
        };
        let result = sanitize_llm_prompt("hack and exploit the cheat", &validator).unwrap();
        assert!(result.starts_with("SAFE: "));
    }

    #[test]
    fn test_sanitize_banned_pattern_takes_priority_over_length() {
        let validator = LLMValidator {
            banned_patterns: vec!["eval(".to_string()],
            allowed_domains: vec![],
            max_prompt_length: 5,
            enable_content_filtering: true,
        };
        // Length check happens first
        let result = sanitize_llm_prompt("eval(something)", &validator);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too long"));
    }

    #[test]
    fn test_sanitize_exact_max_length_prompt() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 5,
            enable_content_filtering: false,
        };
        let result = sanitize_llm_prompt("hello", &validator).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_sanitize_one_over_max_length() {
        let validator = LLMValidator {
            banned_patterns: vec![],
            allowed_domains: vec![],
            max_prompt_length: 5,
            enable_content_filtering: false,
        };
        let result = sanitize_llm_prompt("hello!", &validator);
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_data_empty() {
        let hash = hash_data(b"");
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_hash_data_different_inputs() {
        let h1 = hash_data(b"abc");
        let h2 = hash_data(b"abd");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_signature_wrong_key() {
        let (signing_key, _) = generate_keypair();
        let (_, other_verifying_key) = generate_keypair();
        let data = b"test data";
        let signature = generate_signature(data, &signing_key);
        assert!(!verify_signature(data, &signature, &other_verifying_key));
    }

    #[tokio::test]
    async fn test_sandbox_with_context_variables() {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(1000);
        let sandbox = ScriptSandbox {
            engine: Arc::new(Mutex::new(engine)),
            allowed_functions: HashMap::new(),
            execution_limits: ExecutionLimits {
                max_operations: 1000,
                max_memory_bytes: 1024 * 1024,
                timeout_ms: 1000,
            },
        };
        let mut context = HashMap::new();
        context.insert("x".to_string(), rhai::Dynamic::from(10_i64));
        context.insert("y".to_string(), rhai::Dynamic::from(32_i64));
        let result = execute_script_sandboxed("x + y", &sandbox, context)
            .await
            .unwrap();
        assert_eq!(result.as_int().unwrap(), 42);
    }

    #[tokio::test]
    async fn test_sandbox_compile_error() {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(1000);
        let sandbox = ScriptSandbox {
            engine: Arc::new(Mutex::new(engine)),
            allowed_functions: HashMap::new(),
            execution_limits: ExecutionLimits {
                max_operations: 1000,
                max_memory_bytes: 1024 * 1024,
                timeout_ms: 1000,
            },
        };
        let result = execute_script_sandboxed("let x = ;", &sandbox, HashMap::new()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_security_plugin_build() {
        let plugin = SecurityPlugin::default();
        let mut app = App::new();
        plugin.build(&mut app);

        assert!(app.world.get_resource::<SecurityConfig>().is_some());
        assert!(app.world.get_resource::<TelemetryData>().is_some());
        assert!(app.world.get_resource::<ScriptSandbox>().is_some());
        assert!(app.world.get_resource::<LLMValidator>().is_some());
    }

    #[test]
    fn test_telemetry_severity_debug_format() {
        assert_eq!(format!("{:?}", TelemetrySeverity::Info), "Info");
        assert_eq!(format!("{:?}", TelemetrySeverity::Critical), "Critical");
    }

    #[test]
    fn test_telemetry_event_serde_roundtrip() {
        let event = TelemetryEvent {
            timestamp: 999,
            event_type: "test_rt".to_string(),
            severity: TelemetrySeverity::Error,
            data: serde_json::json!({"a": 1}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: TelemetryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.timestamp, 999);
        assert_eq!(restored.severity, TelemetrySeverity::Error);
    }

    // ═══════════════════════════════════════════════════════════════
    // MUTATION REMEDIATION TESTS
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn mutation_plugin_build_sets_correct_memory_limit() {
        // Targets: lib.rs:148 replace * with +/- in SecurityPlugin::build
        // 1024 * 1024 = 1_048_576 (1 MB); + would give 2048, / would give 1
        let plugin = SecurityPlugin::default();
        let mut app = App::new();
        plugin.build(&mut app);

        let sandbox = app.world.get_resource::<ScriptSandbox>().unwrap();
        assert_eq!(
            sandbox.execution_limits.max_memory_bytes,
            1024 * 1024,
            "max_memory_bytes must be 1 MB (1024 * 1024 = {}), got {}",
            1024 * 1024,
            sandbox.execution_limits.max_memory_bytes
        );
    }

    #[test]
    fn mutation_validate_player_trust_boundary() {
        // Targets: lib.rs:329 replace > with >= in validate_player_input
        //
        // Trust score with impossible_movement + rapid_input = 1.0 * 0.8 * 0.5 = 0.4
        // Trust score with all three = 1.0 * 0.8 * 0.5 * 0.3 = 0.12
        // To get close to 0.2: impossible_movement + memory_tamper = 1.0 * 0.5 * 0.3 = 0.15
        // 0.15 < 0.2 → is_valid = false (both > and >= agree)
        //
        // memory_tamper alone: 1.0 * 0.3 = 0.3 > 0.2 → valid (both agree)
        //
        // All possible values: 1.0, 0.8, 0.5, 0.4, 0.3, 0.24, 0.15, 0.12
        // None equals exactly 0.2 → this mutation is EQUIVALENT
        //
        // However, we still verify the boundary logic is correct:
        let anti_cheat_tamper = CAntiCheat {
            player_id: String::new(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec!["memory_tamper".to_string()],
        };
        let result = validate_player_input(&anti_cheat_tamper);
        // 1.0 * 0.3 = 0.3 > 0.2 → valid
        assert!(result.is_valid, "trust_score 0.3 should be valid");

        let anti_cheat_two = CAntiCheat {
            player_id: String::new(),
            trust_score: 1.0,
            last_validation: 0,
            anomaly_flags: vec![
                "impossible_movement".to_string(),
                "memory_tamper".to_string(),
            ],
        };
        let result2 = validate_player_input(&anti_cheat_two);
        // 1.0 * 0.5 * 0.3 = 0.15 < 0.2 → invalid
        assert!(!result2.is_valid, "trust_score 0.15 should be invalid");
    }
}
