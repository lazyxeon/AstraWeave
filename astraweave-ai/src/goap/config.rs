// TOML configuration system for GOAP learning and persistence
// Phase 3: Learning & Persistence

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GOAPConfig {
    pub learning: LearningConfig,
    pub cost_tuning: CostTuningConfig,
    pub persistence: PersistenceConfig,
    pub telemetry: TelemetryConfig,
    pub planner: PlannerConfig,
    pub debug: DebugConfig,
}

impl Default for GOAPConfig {
    fn default() -> Self {
        Self {
            learning: LearningConfig::default(),
            cost_tuning: CostTuningConfig::default(),
            persistence: PersistenceConfig::default(),
            telemetry: TelemetryConfig::default(),
            planner: PlannerConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    pub enabled: bool,
    pub initial_success_rate: f32,
    pub min_success_rate: f32,
    pub max_success_rate: f32,
    pub smoothing: SmoothingConfig,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_success_rate: 0.75,
            min_success_rate: 0.1,
            max_success_rate: 0.95,
            smoothing: SmoothingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmoothingConfig {
    pub method: SmoothingMethod,
    pub ewma_alpha: f32,
    pub bayesian_prior_successes: u32,
    pub bayesian_prior_failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SmoothingMethod {
    Ewma,
    Bayesian,
}

impl Default for SmoothingConfig {
    fn default() -> Self {
        Self {
            method: SmoothingMethod::Ewma,
            ewma_alpha: 0.2,
            bayesian_prior_successes: 3,
            bayesian_prior_failures: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTuningConfig {
    pub base_cost_multiplier: f32,
    pub risk_weight: f32,
    pub health_critical_threshold: i32,
    pub health_wounded_threshold: i32,
    pub ammo_critical_threshold: i32,
    pub ammo_low_threshold: i32,
    pub morale_low_threshold: f32,
    pub morale_high_threshold: f32,
}

impl Default for CostTuningConfig {
    fn default() -> Self {
        Self {
            base_cost_multiplier: 1.0,
            risk_weight: 5.0,
            health_critical_threshold: 30,
            health_wounded_threshold: 60,
            ammo_critical_threshold: 5,
            ammo_low_threshold: 10,
            morale_low_threshold: 0.4,
            morale_high_threshold: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    pub enabled: bool,
    pub format: PersistenceFormat,
    pub save_interval_seconds: u64,
    pub retention_days: u32,
    pub max_entries_per_action: usize,
    pub prune_threshold: usize,
    pub file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum PersistenceFormat {
    Json,
    Bincode,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: PersistenceFormat::Json,
            save_interval_seconds: 60,
            retention_days: 30,
            max_entries_per_action: 1000,
            prune_threshold: 10000,
            file_path: "saves/goap_history.json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub max_events: usize,
    pub export_enabled: bool,
    pub export_path: String,
    pub export_interval_seconds: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_events: 1000,
            export_enabled: false,
            export_path: "telemetry/goap_events.json".to_string(),
            export_interval_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    pub max_iterations: usize,
    pub max_plan_length: usize,
    pub multi_goal_enabled: bool,
    pub max_concurrent_goals: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            max_plan_length: 20,
            multi_goal_enabled: true,
            max_concurrent_goals: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    pub verbose: bool,
    pub log_planning: bool,
    pub log_execution: bool,
    pub log_learning: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            log_planning: false,
            log_execution: true,
            log_learning: true,
        }
    }
}

/// Configuration loading and validation
impl GOAPConfig {
    /// Load config from TOML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::ReadFailed(e.to_string()))?;

        let config: GOAPConfig =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseFailed(e.to_string()))?;

        config.validate()?;
        Ok(config)
    }

    /// Load config with fallback to default
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        match Self::load(path.as_ref()) {
            Ok(config) => {
                tracing::info!("Loaded GOAP config from {:?}", path.as_ref());
                config
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load GOAP config from {:?}: {}. Using defaults.",
                    path.as_ref(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate learning config
        if self.learning.initial_success_rate < 0.0 || self.learning.initial_success_rate > 1.0 {
            return Err(ConfigError::InvalidValue(
                "learning.initial_success_rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.learning.min_success_rate < 0.0 || self.learning.min_success_rate > 1.0 {
            return Err(ConfigError::InvalidValue(
                "learning.min_success_rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.learning.max_success_rate < 0.0 || self.learning.max_success_rate > 1.0 {
            return Err(ConfigError::InvalidValue(
                "learning.max_success_rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        if self.learning.min_success_rate >= self.learning.max_success_rate {
            return Err(ConfigError::InvalidValue(
                "learning.min_success_rate must be less than max_success_rate".to_string(),
            ));
        }

        // Validate EWMA alpha
        if self.learning.smoothing.ewma_alpha < 0.0 || self.learning.smoothing.ewma_alpha > 1.0 {
            return Err(ConfigError::InvalidValue(
                "learning.smoothing.ewma_alpha must be between 0.0 and 1.0".to_string(),
            ));
        }

        // Validate cost tuning
        if self.cost_tuning.base_cost_multiplier <= 0.0 {
            return Err(ConfigError::InvalidValue(
                "cost_tuning.base_cost_multiplier must be positive".to_string(),
            ));
        }

        if self.cost_tuning.risk_weight < 0.0 {
            return Err(ConfigError::InvalidValue(
                "cost_tuning.risk_weight must be non-negative".to_string(),
            ));
        }

        // Validate thresholds
        if self.cost_tuning.health_critical_threshold >= self.cost_tuning.health_wounded_threshold {
            return Err(ConfigError::InvalidValue(
                "health_critical_threshold must be less than health_wounded_threshold".to_string(),
            ));
        }

        if self.cost_tuning.ammo_critical_threshold >= self.cost_tuning.ammo_low_threshold {
            return Err(ConfigError::InvalidValue(
                "ammo_critical_threshold must be less than ammo_low_threshold".to_string(),
            ));
        }

        // Validate morale thresholds
        if self.cost_tuning.morale_low_threshold >= self.cost_tuning.morale_high_threshold {
            return Err(ConfigError::InvalidValue(
                "morale_low_threshold must be less than morale_high_threshold".to_string(),
            ));
        }

        // Validate persistence config
        if self.persistence.max_entries_per_action == 0 {
            return Err(ConfigError::InvalidValue(
                "persistence.max_entries_per_action must be positive".to_string(),
            ));
        }

        // Validate planner config
        if self.planner.max_iterations == 0 {
            return Err(ConfigError::InvalidValue(
                "planner.max_iterations must be positive".to_string(),
            ));
        }

        if self.planner.max_plan_length == 0 {
            return Err(ConfigError::InvalidValue(
                "planner.max_plan_length must be positive".to_string(),
            ));
        }

        if self.planner.max_concurrent_goals == 0 {
            return Err(ConfigError::InvalidValue(
                "planner.max_concurrent_goals must be positive".to_string(),
            ));
        }

        Ok(())
    }

    /// Save config to TOML file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::SerializeFailed(e.to_string()))?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::WriteFailed(e.to_string()))?;
        }

        fs::write(path.as_ref(), contents).map_err(|e| ConfigError::WriteFailed(e.to_string()))?;

        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, Clone)]
#[non_exhaustive]
#[must_use]
pub enum ConfigError {
    ReadFailed(String),
    ParseFailed(String),
    WriteFailed(String),
    SerializeFailed(String),
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed(msg) => write!(f, "Failed to read config: {}", msg),
            Self::ParseFailed(msg) => write!(f, "Failed to parse config: {}", msg),
            Self::WriteFailed(msg) => write!(f, "Failed to write config: {}", msg),
            Self::SerializeFailed(msg) => write!(f, "Failed to serialize config: {}", msg),
            Self::InvalidValue(msg) => write!(f, "Invalid config value: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_is_valid() {
        let config = GOAPConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let original = GOAPConfig::default();
        original.save(&file_path).expect("Failed to save config");

        let loaded = GOAPConfig::load(&file_path).expect("Failed to load config");

        // Verify key values match
        assert_eq!(loaded.learning.enabled, original.learning.enabled);
        assert_eq!(
            loaded.cost_tuning.risk_weight,
            original.cost_tuning.risk_weight
        );
        assert_eq!(loaded.persistence.format, original.persistence.format);
    }

    #[test]
    fn test_load_or_default_missing_file() {
        let config = GOAPConfig::load_or_default("nonexistent.toml");

        // Should return default without panicking
        assert_eq!(config.learning.initial_success_rate, 0.75);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_success_rate_validation() {
        let mut config = GOAPConfig::default();
        config.learning.initial_success_rate = 1.5; // Invalid

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_threshold_validation() {
        let mut config = GOAPConfig::default();
        config.cost_tuning.health_critical_threshold = 70;
        config.cost_tuning.health_wounded_threshold = 60; // Critical > wounded (invalid)

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        let mut original = GOAPConfig::default();
        original.learning.smoothing.ewma_alpha = 0.3;
        original.cost_tuning.risk_weight = 7.5;

        original.save(&file_path).unwrap();
        let loaded = GOAPConfig::load(&file_path).unwrap();

        assert_eq!(loaded.learning.smoothing.ewma_alpha, 0.3);
        assert_eq!(loaded.cost_tuning.risk_weight, 7.5);
    }

    #[test]
    fn test_smoothing_method_serialization() {
        let config = SmoothingConfig {
            method: SmoothingMethod::Bayesian,
            ewma_alpha: 0.2,
            bayesian_prior_successes: 5,
            bayesian_prior_failures: 2,
        };

        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("method = \"bayesian\""));

        let deserialized: SmoothingConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.method, SmoothingMethod::Bayesian);
    }

    // ================================================================
    // Mutation-killing tests
    // ================================================================

    /// Kills line 215: `load_or_default` with valid file returns loaded config
    #[test]
    fn test_load_or_default_with_valid_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("valid.toml");

        let mut config = GOAPConfig::default();
        config.cost_tuning.risk_weight = 42.0;
        config.save(&path).unwrap();

        let loaded = GOAPConfig::load_or_default(&path);
        assert_eq!(
            loaded.cost_tuning.risk_weight, 42.0,
            "load_or_default should return loaded config, not default"
        );
    }

    /// Kills lines 234: boundary tests for initial_success_rate
    /// Mutations: `< 0.0` → `<= 0.0`, `< 0.0` → `== 0.0`, `> 1.0` → `>= 1.0`
    #[test]
    fn test_validate_initial_success_rate_boundaries() {
        let mut config = GOAPConfig::default();

        // Exactly 0.0 should be valid (kills < → <=)
        config.learning.initial_success_rate = 0.0;
        assert!(
            config.validate().is_ok(),
            "initial_success_rate=0.0 should be valid"
        );

        // Exactly 1.0 should be valid (kills > → >=)
        config.learning.initial_success_rate = 1.0;
        assert!(
            config.validate().is_ok(),
            "initial_success_rate=1.0 should be valid"
        );

        // Negative should fail (kills < → ==)
        config.learning.initial_success_rate = -0.1;
        assert!(
            config.validate().is_err(),
            "initial_success_rate=-0.1 should be invalid"
        );
    }

    /// Kills lines 240: boundary tests for min_success_rate
    #[test]
    fn test_validate_min_success_rate_boundaries() {
        let mut config = GOAPConfig::default();

        config.learning.min_success_rate = 0.0;
        assert!(
            config.validate().is_ok(),
            "min_success_rate=0.0 should be valid"
        );

        config.learning.min_success_rate = -0.1;
        assert!(
            config.validate().is_err(),
            "min_success_rate=-0.1 should be invalid"
        );

        // Just under max to avoid min >= max error
        config.learning.min_success_rate = config.learning.max_success_rate - 0.01;
        assert!(config.validate().is_ok());
    }

    /// Kills lines 246: boundary tests for max_success_rate
    #[test]
    fn test_validate_max_success_rate_boundaries() {
        let mut config = GOAPConfig::default();

        config.learning.max_success_rate = 1.0;
        assert!(
            config.validate().is_ok(),
            "max_success_rate=1.0 should be valid"
        );

        config.learning.max_success_rate = -0.1;
        assert!(
            config.validate().is_err(),
            "max_success_rate=-0.1 should be invalid"
        );

        config.learning.max_success_rate = 1.1;
        assert!(
            config.validate().is_err(),
            "max_success_rate=1.1 should be invalid"
        );
    }

    /// Kills lines 259: boundary tests for ewma_alpha
    #[test]
    fn test_validate_ewma_alpha_boundaries() {
        let mut config = GOAPConfig::default();

        config.learning.smoothing.ewma_alpha = 0.0;
        assert!(config.validate().is_ok(), "ewma_alpha=0.0 should be valid");

        config.learning.smoothing.ewma_alpha = 1.0;
        assert!(config.validate().is_ok(), "ewma_alpha=1.0 should be valid");

        config.learning.smoothing.ewma_alpha = -0.1;
        assert!(
            config.validate().is_err(),
            "ewma_alpha=-0.1 should be invalid"
        );

        config.learning.smoothing.ewma_alpha = 1.1;
        assert!(
            config.validate().is_err(),
            "ewma_alpha=1.1 should be invalid"
        );
    }

    /// Kills line 272: boundary test for risk_weight
    #[test]
    fn test_validate_risk_weight_boundary() {
        let mut config = GOAPConfig::default();

        config.cost_tuning.risk_weight = 0.0;
        assert!(config.validate().is_ok(), "risk_weight=0.0 should be valid");

        config.cost_tuning.risk_weight = -0.1;
        assert!(
            config.validate().is_err(),
            "risk_weight=-0.1 should be invalid"
        );
    }
}
