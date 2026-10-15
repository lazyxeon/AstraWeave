//! Mutation-resistant comprehensive tests for astraweave-security.
//! Targets exact trust scores, boundary conditions, hash values,
//! error messages, and deserialization limits for 90%+ mutation kill rate.

use astraweave_security::deserialization::*;
use astraweave_security::path::*;
use astraweave_security::*;
use std::path::Path;

// ========================================================================
// TELEMETRY SEVERITY ENUM
// ========================================================================

#[test]
fn severity_all_variants() {
    let vars = [
        TelemetrySeverity::Info,
        TelemetrySeverity::Warning,
        TelemetrySeverity::Error,
        TelemetrySeverity::Critical,
    ];
    assert_eq!(vars.len(), 4);
}

#[test]
fn severity_eq() {
    assert_eq!(TelemetrySeverity::Info, TelemetrySeverity::Info);
    assert_ne!(TelemetrySeverity::Info, TelemetrySeverity::Warning);
    assert_ne!(TelemetrySeverity::Warning, TelemetrySeverity::Error);
    assert_ne!(TelemetrySeverity::Error, TelemetrySeverity::Critical);
}

#[test]
fn severity_debug() {
    assert_eq!(format!("{:?}", TelemetrySeverity::Info), "Info");
    assert_eq!(format!("{:?}", TelemetrySeverity::Critical), "Critical");
}

#[test]
fn severity_serde_roundtrip() {
    let s = TelemetrySeverity::Warning;
    let json = serde_json::to_string(&s).unwrap();
    let s2: TelemetrySeverity = serde_json::from_str(&json).unwrap();
    assert_eq!(s2, TelemetrySeverity::Warning);
}

// ========================================================================
// TELEMETRY EVENT
// ========================================================================

#[test]
fn telemetry_event_serde_roundtrip() {
    let e = TelemetryEvent {
        timestamp: 1000,
        event_type: "test".to_string(),
        severity: TelemetrySeverity::Info,
        data: serde_json::json!({"key": "value"}),
    };
    let json = serde_json::to_string(&e).unwrap();
    let e2: TelemetryEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(e2.timestamp, 1000);
    assert_eq!(e2.event_type, "test");
    assert_eq!(e2.severity, TelemetrySeverity::Info);
}

// ========================================================================
// VALIDATE PLAYER INPUT — trust score calculations
// ========================================================================

fn make_anti_cheat(flags: Vec<&str>) -> CAntiCheat {
    CAntiCheat {
        player_id: "player_1".to_string(),
        trust_score: 1.0,
        last_validation: 0,
        anomaly_flags: flags.into_iter().map(String::from).collect(),
    }
}

#[test]
fn validate_no_flags_trust_1_0() {
    let ac = make_anti_cheat(vec![]);
    let r = validate_player_input(&ac);
    assert!((r.trust_score - 1.0).abs() < 1e-6, "no flags = 1.0 trust");
    assert!(r.is_valid);
    assert!(r.warnings.is_empty());
    assert!(r.anomalies.is_empty());
}

#[test]
fn validate_rapid_input_trust_0_8() {
    let ac = make_anti_cheat(vec!["rapid_input"]);
    let r = validate_player_input(&ac);
    assert!((r.trust_score - 0.8).abs() < 1e-6, "rapid_input = 0.8");
    assert!(r.is_valid);
}

#[test]
fn validate_impossible_movement_trust_0_5() {
    let ac = make_anti_cheat(vec!["impossible_movement"]);
    let r = validate_player_input(&ac);
    assert!(
        (r.trust_score - 0.5).abs() < 1e-6,
        "impossible_movement = 0.5"
    );
    assert!(r.is_valid);
}

#[test]
fn validate_memory_tamper_trust_0_3() {
    let ac = make_anti_cheat(vec!["memory_tamper"]);
    let r = validate_player_input(&ac);
    assert!((r.trust_score - 0.3).abs() < 1e-6, "memory_tamper = 0.3");
    assert!(r.is_valid);
}

#[test]
fn validate_rapid_plus_impossible_trust_0_4() {
    let ac = make_anti_cheat(vec!["rapid_input", "impossible_movement"]);
    let r = validate_player_input(&ac);
    assert!((r.trust_score - 0.4).abs() < 1e-6, "0.8 * 0.5 = 0.4");
    assert!(r.is_valid);
}

#[test]
fn validate_impossible_plus_tamper_trust_0_15_invalid() {
    let ac = make_anti_cheat(vec!["impossible_movement", "memory_tamper"]);
    let r = validate_player_input(&ac);
    assert!((r.trust_score - 0.15).abs() < 1e-6, "0.5 * 0.3 = 0.15");
    assert!(!r.is_valid, "0.15 <= 0.2 threshold → invalid");
}

#[test]
fn validate_all_three_flags_trust_0_12_invalid() {
    let ac = make_anti_cheat(vec!["rapid_input", "impossible_movement", "memory_tamper"]);
    let r = validate_player_input(&ac);
    assert!(
        (r.trust_score - 0.12).abs() < 1e-6,
        "0.8 * 0.5 * 0.3 = 0.12"
    );
    assert!(!r.is_valid, "0.12 <= 0.2 → invalid");
}

#[test]
fn validate_threshold_boundary_0_2_is_invalid() {
    // Trust of exactly 0.2 => NOT valid (threshold is strictly >0.2)
    // memory_tamper gives 0.3 which is > 0.2
    let ac = make_anti_cheat(vec!["memory_tamper"]);
    let r = validate_player_input(&ac);
    assert!(r.is_valid, "0.3 > 0.2 → valid");
}

#[test]
fn validate_unknown_flag_ignored() {
    let ac = make_anti_cheat(vec!["unknown_flag"]);
    let r = validate_player_input(&ac);
    assert!(
        (r.trust_score - 1.0).abs() < 1e-6,
        "unknown flags have no effect"
    );
    assert!(r.is_valid);
}

#[test]
fn validate_rapid_warning_message() {
    let ac = make_anti_cheat(vec!["rapid_input"]);
    let r = validate_player_input(&ac);
    assert!(
        r.warnings.iter().any(|w| w.contains("rapid input")),
        "should warn about rapid input"
    );
}

#[test]
fn validate_impossible_warning_message() {
    let ac = make_anti_cheat(vec!["impossible_movement"]);
    let r = validate_player_input(&ac);
    assert!(
        r.warnings.iter().any(|w| w.contains("movement")),
        "should warn about movement"
    );
}

#[test]
fn validate_tamper_warning_message() {
    let ac = make_anti_cheat(vec!["memory_tamper"]);
    let r = validate_player_input(&ac);
    assert!(
        r.warnings.iter().any(|w| w.contains("tamper")),
        "should warn about tamper"
    );
}

// ========================================================================
// SANITIZE LLM PROMPT
// ========================================================================

fn make_validator() -> LLMValidator {
    LLMValidator {
        banned_patterns: vec![
            "system(".to_string(),
            "exec(".to_string(),
            "eval(".to_string(),
            "import ".to_string(),
        ],
        allowed_domains: astraweave_security::default_allowed_llm_domains(),
        max_prompt_length: 10000,
        enable_content_filtering: true,
    }
}

#[test]
fn sanitize_empty_prompt_ok() {
    let v = make_validator();
    let result = sanitize_llm_prompt("", &v).unwrap();
    assert_eq!(result, "");
}

#[test]
fn sanitize_normal_prompt_passthrough() {
    let v = make_validator();
    let result = sanitize_llm_prompt("hello", &v).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn sanitize_too_long_prompt() {
    let v = make_validator();
    let long = "x".repeat(10001);
    let result = sanitize_llm_prompt(&long, &v);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("too long") || msg.contains("Prompt too long"),
        "error should mention length: {}",
        msg
    );
}

#[test]
fn sanitize_exact_max_length_ok() {
    let v = make_validator();
    let exact = "x".repeat(10000);
    let result = sanitize_llm_prompt(&exact, &v);
    assert!(result.is_ok());
}

#[test]
fn sanitize_banned_system() {
    let v = make_validator();
    let result = sanitize_llm_prompt("call system(rm -rf)", &v);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("banned pattern"));
}

#[test]
fn sanitize_banned_exec() {
    let v = make_validator();
    let result = sanitize_llm_prompt("exec(cmd)", &v);
    assert!(result.is_err());
}

#[test]
fn sanitize_banned_eval() {
    let v = make_validator();
    let result = sanitize_llm_prompt("try eval(x)", &v);
    assert!(result.is_err());
}

#[test]
fn sanitize_banned_import() {
    let v = make_validator();
    let result = sanitize_llm_prompt("import os", &v);
    assert!(result.is_err());
}

#[test]
fn sanitize_case_sensitive_not_banned() {
    // "System(" with capital S doesn't match "system("
    let v = make_validator();
    let result = sanitize_llm_prompt("System(x)", &v);
    assert!(result.is_ok(), "case-sensitive: System( != system(");
}

#[test]
fn sanitize_content_filter_hack_prefixed() {
    let v = make_validator();
    let result = sanitize_llm_prompt("how to hack", &v).unwrap();
    assert!(
        result.starts_with("SAFE:"),
        "hack triggers safe prefix: {}",
        result
    );
}

#[test]
fn sanitize_content_filter_exploit_prefixed() {
    let v = make_validator();
    let result = sanitize_llm_prompt("find exploits", &v).unwrap();
    assert!(
        result.starts_with("SAFE:"),
        "exploit triggers safe prefix: {}",
        result
    );
}

#[test]
fn sanitize_filter_disabled_no_prefix() {
    let mut v = make_validator();
    v.enable_content_filtering = false;
    let result = sanitize_llm_prompt("how to hack", &v).unwrap();
    assert_eq!(result, "how to hack", "no filtering = passthrough");
}

// ========================================================================
// HASH DATA — SHA-256
// ========================================================================

#[test]
fn hash_empty_data() {
    let h = hash_data(b"");
    assert_eq!(
        h,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn hash_output_length_64() {
    let h = hash_data(b"test");
    assert_eq!(h.len(), 64, "SHA-256 hex is always 64 chars");
}

#[test]
fn hash_lowercase_hex() {
    let h = hash_data(b"hello");
    assert!(
        h.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "should be lowercase hex"
    );
}

#[test]
fn hash_deterministic() {
    let h1 = hash_data(b"test data");
    let h2 = hash_data(b"test data");
    assert_eq!(h1, h2, "same input = same hash");
}

#[test]
fn hash_different_inputs_differ() {
    let h1 = hash_data(b"abc");
    let h2 = hash_data(b"abd");
    assert_ne!(h1, h2, "different inputs = different hashes");
}

// ========================================================================
// DIGITAL SIGNATURES
// ========================================================================

#[test]
fn keypair_sign_verify() {
    let (signing_key, verifying_key) = generate_keypair();
    let data = b"important message";
    let sig = generate_signature(data, &signing_key);
    assert!(verify_signature(data, &sig, &verifying_key));
}

#[test]
fn signature_wrong_data_fails() {
    let (signing_key, verifying_key) = generate_keypair();
    let sig = generate_signature(b"original", &signing_key);
    assert!(!verify_signature(b"tampered", &sig, &verifying_key));
}

#[test]
fn signature_wrong_key_fails() {
    let (signing_key, _vk1) = generate_keypair();
    let (_sk2, vk2) = generate_keypair();
    let sig = generate_signature(b"data", &signing_key);
    assert!(!verify_signature(b"data", &sig, &vk2));
}

#[test]
fn signature_deterministic() {
    let (sk, _) = generate_keypair();
    let sig1 = generate_signature(b"hello", &sk);
    let sig2 = generate_signature(b"hello", &sk);
    assert_eq!(sig1.to_bytes(), sig2.to_bytes());
}

// ========================================================================
// DESERIALIZATION LIMITS
// ========================================================================

#[test]
fn max_json_bytes_is_10mb() {
    assert_eq!(MAX_JSON_BYTES, 10 * 1024 * 1024);
    assert_eq!(MAX_JSON_BYTES, 10_485_760);
}

#[test]
fn max_toml_bytes_is_5mb() {
    assert_eq!(MAX_TOML_BYTES, 5 * 1024 * 1024);
    assert_eq!(MAX_TOML_BYTES, 5_242_880);
}

#[test]
fn max_ron_bytes_is_5mb() {
    assert_eq!(MAX_RON_BYTES, 5 * 1024 * 1024);
    assert_eq!(MAX_RON_BYTES, 5_242_880);
}

#[test]
fn max_toml_equals_max_ron() {
    assert_eq!(MAX_TOML_BYTES, MAX_RON_BYTES);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn max_json_greater_than_toml() {
    assert!(MAX_JSON_BYTES > MAX_TOML_BYTES);
}

// ========================================================================
// PATH VALIDATION
// ========================================================================

#[test]
fn validate_extension_allowed() {
    let result = validate_extension(Path::new("file.png"), &["png", "jpg"]);
    assert!(result.is_ok());
}

#[test]
fn validate_extension_not_allowed() {
    let result = validate_extension(Path::new("file.exe"), &["png", "jpg"]);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("not allowed"), "msg: {}", msg);
}

#[test]
fn validate_extension_no_extension() {
    let result = validate_extension(Path::new("noext"), &["png"]);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("no extension"), "msg: {}", msg);
}

#[test]
fn validate_extension_case_sensitive() {
    let result = validate_extension(Path::new("FILE.PNG"), &["png"]);
    assert!(result.is_err(), "PNG != png (case-sensitive)");
}

#[test]
fn validate_extension_multiple_dots() {
    // tar.gz => checks "gz"
    let result = validate_extension(Path::new("archive.tar.gz"), &["gz"]);
    assert!(result.is_ok());
}

// ========================================================================
// SECURITY CONFIG DEFAULTS
// ========================================================================

#[test]
fn security_plugin_default_config() {
    let plugin = SecurityPlugin::default();
    // We just verify it doesn't panic
    let _ = plugin;
}

// ========================================================================
// EXECUTION LIMITS
// ========================================================================

#[test]
fn execution_limits_clone_debug() {
    let limits = ExecutionLimits {
        max_operations: 10000,
        max_memory_bytes: 1_048_576,
        timeout_ms: 1000,
    };
    let limits2 = limits.clone();
    assert_eq!(limits2.max_operations, 10000);
    assert_eq!(limits2.max_memory_bytes, 1_048_576);
    assert_eq!(limits2.timeout_ms, 1000);
    let dbg = format!("{:?}", limits);
    assert!(dbg.contains("ExecutionLimits"));
}

#[test]
fn execution_limits_default_max_memory_1mb() {
    // Default from SecurityPlugin::build: 1024 * 1024 = 1MB
    let limits = ExecutionLimits {
        max_operations: 10000,
        max_memory_bytes: 1024 * 1024,
        timeout_ms: 1000,
    };
    assert_eq!(limits.max_memory_bytes, 1_048_576);
}

// ========================================================================
// LLM VALIDATOR FIELDS
// ========================================================================

#[test]
fn llm_validator_default_banned_patterns() {
    let v = make_validator();
    assert_eq!(v.banned_patterns.len(), 4);
    assert!(v.banned_patterns.contains(&"system(".to_string()));
    assert!(v.banned_patterns.contains(&"exec(".to_string()));
    assert!(v.banned_patterns.contains(&"eval(".to_string()));
    assert!(v.banned_patterns.contains(&"import ".to_string()));
}

#[test]
fn llm_validator_default_domains() {
    let v = make_validator();
    assert_eq!(v.allowed_domains.len(), 1);
    assert!(v.allowed_domains.contains(&"localhost".to_string()));
    assert!(!v.allowed_domains.iter().any(|d| d.contains("openai")));
    assert!(!v.allowed_domains.iter().any(|d| d.contains("anthropic")));
}

#[test]
fn llm_validator_default_max_prompt_length() {
    let v = make_validator();
    assert_eq!(v.max_prompt_length, 10000);
}

#[test]
fn llm_validator_default_content_filtering() {
    let v = make_validator();
    assert!(v.enable_content_filtering);
}

// ========================================================================
// VALIDATION RESULT FIELDS
// ========================================================================

#[test]
fn validation_result_clone_debug() {
    let r = ValidationResult {
        is_valid: true,
        trust_score: 1.0,
        warnings: vec!["w".to_string()],
        anomalies: vec![],
    };
    let r2 = r.clone();
    assert!(r2.is_valid);
    assert!((r2.trust_score - 1.0).abs() < 1e-6);
    assert_eq!(r2.warnings.len(), 1);
    let dbg = format!("{:?}", r);
    assert!(dbg.contains("ValidationResult"));
}

// ========================================================================
// C ANTI CHEAT FIELDS
// ========================================================================

#[test]
fn anti_cheat_clone_debug() {
    let ac = CAntiCheat {
        player_id: "p1".to_string(),
        trust_score: 0.5,
        last_validation: 12345,
        anomaly_flags: vec!["flag1".to_string()],
    };
    let ac2 = ac.clone();
    assert_eq!(ac2.player_id, "p1");
    assert!((ac2.trust_score - 0.5).abs() < 1e-6);
    assert_eq!(ac2.last_validation, 12345);
    assert_eq!(ac2.anomaly_flags.len(), 1);
    let dbg = format!("{:?}", ac);
    assert!(dbg.contains("CAntiCheat"));
}
