#![forbid(unsafe_code)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{error, info, Level};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use astraweave_ecs::{App, Plugin};

pub mod llm_telemetry;
pub use llm_telemetry::*;

mod companion;
pub use companion::*;

/// Configuration for observability stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub tracing_level: String,
    pub metrics_enabled: bool,
    pub crash_reporting_enabled: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            tracing_level: "INFO".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: true,
        }
    }
}

/// Resource for observability state
pub struct ObservabilityState {
    pub config: ObservabilityConfig,
}

impl ObservabilityState {
    pub fn new(config: ObservabilityConfig) -> Self {
        Self { config }
    }
}

/// Plugin for observability integration
pub struct ObservabilityPlugin {
    config: ObservabilityConfig,
}

impl ObservabilityPlugin {
    pub fn new(config: ObservabilityConfig) -> Self {
        Self { config }
    }
}

impl Default for ObservabilityPlugin {
    fn default() -> Self {
        Self::new(ObservabilityConfig::default())
    }
}

impl Plugin for ObservabilityPlugin {
    #[allow(clippy::expect_used)] // Plugin initialization failure is fatal — no recovery possible
    fn build(&self, app: &mut App) {
        // Initialize tracing
        init_tracing(&self.config).expect("Failed to initialize tracing");

        // Initialize metrics if enabled
        if self.config.metrics_enabled {
            init_metrics(&self.config).expect("Failed to initialize metrics");
        }

        // Initialize crash reporting if enabled
        if self.config.crash_reporting_enabled {
            init_crash_reporting();
        }

        // Add observability state as resource
        app.world
            .insert_resource(ObservabilityState::new(self.config.clone()));

        // Add observability systems
        app.add_system_known_stage("presentation", observability_system);
    }
}

/// Initialize tracing with JSON output and filtering
fn init_tracing(config: &ObservabilityConfig) -> Result<()> {
    use std::sync::Once;
    static TRACING_INIT: Once = Once::new();

    let level = match config.tracing_level.as_str() {
        "TRACE" => Level::TRACE,
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        _ => Level::INFO,
    };

    let config_level = config.tracing_level.clone();

    // Only initialize once per process (safe for tests and repeated calls)
    TRACING_INIT.call_once(|| {
        let subscriber = tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::from_default_env().add_directive(level.into()))
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_target(false)
                    .with_thread_ids(true)
                    .with_thread_names(true),
            );

        // Use try_init to avoid panic if already initialized
        let _ = subscriber.try_init();
    });

    info!("Tracing initialized with level: {}", config_level);
    Ok(())
}

/// Initialize metrics with simple recorder
fn init_metrics(_config: &ObservabilityConfig) -> Result<()> {
    // For now, use a simple recorder that just logs metrics
    // In production, this could be extended to export to various backends
    info!("Metrics initialized with simple recorder");
    Ok(())
}

/// Initialize basic crash reporting (logs panics)
fn init_crash_reporting() {
    use std::sync::Once;
    static CRASH_INIT: Once = Once::new();

    CRASH_INIT.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            let backtrace = std::backtrace::Backtrace::capture();
            error!("Panic occurred: {}\nBacktrace:\n{}", panic_info, backtrace);

            // In a real implementation, this would send to a crash reporting service
            // like Sentry, but for now we just log it
        }));
    });

    info!("Crash reporting initialized");
}

/// System that collects observability metrics
fn observability_system(world: &mut astraweave_ecs::World) {
    if let Some(state) = world.get_resource::<ObservabilityState>() {
        if state.config.metrics_enabled {
            // For now, just log metrics instead of using metrics crate
            info!("Tick recorded");
        }
    }
}

/// Convenience function to initialize full observability stack
pub fn init_observability(config: ObservabilityConfig) -> Result<()> {
    init_tracing(&config)?;
    if config.metrics_enabled {
        init_metrics(&config)?;
    }
    if config.crash_reporting_enabled {
        init_crash_reporting();
    }
    Ok(())
}

/// Macros for common observability patterns
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::TRACE, $name)
    };
}

#[macro_export]
macro_rules! debug_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::DEBUG, $name)
    };
}

#[macro_export]
macro_rules! info_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::INFO, $name)
    };
}

#[macro_export]
macro_rules! warn_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::WARN, $name)
    };
}

#[macro_export]
macro_rules! error_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::ERROR, $name)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use astraweave_ecs::App;

    #[test]
    fn test_observability_config_default() {
        let config = ObservabilityConfig::default();
        assert_eq!(config.tracing_level, "INFO");
        assert!(config.metrics_enabled);
        assert!(config.crash_reporting_enabled);
    }

    #[test]
    fn test_plugin_build() {
        let mut app = App::new();
        let plugin = ObservabilityPlugin::default();
        plugin.build(&mut app);

        // Check that the resource was inserted
        assert!(app.world.get_resource::<ObservabilityState>().is_some());
    }

    #[test]
    fn test_observability_config_custom() {
        let config = ObservabilityConfig {
            tracing_level: "DEBUG".to_string(),
            metrics_enabled: false,
            crash_reporting_enabled: false,
        };
        assert_eq!(config.tracing_level, "DEBUG");
        assert!(!config.metrics_enabled);
        assert!(!config.crash_reporting_enabled);
    }

    #[test]
    fn test_observability_state_new() {
        let config = ObservabilityConfig::default();
        let state = ObservabilityState::new(config);
        assert_eq!(state.config.tracing_level, "INFO");
    }

    #[test]
    fn test_observability_plugin_new() {
        let config = ObservabilityConfig {
            tracing_level: "WARN".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: false,
        };
        let plugin = ObservabilityPlugin::new(config);
        assert_eq!(plugin.config.tracing_level, "WARN");
    }

    #[test]
    fn test_init_observability_success() {
        let config = ObservabilityConfig::default();
        let result = init_observability(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_observability_metrics_disabled() {
        let config = ObservabilityConfig {
            tracing_level: "ERROR".to_string(),
            metrics_enabled: false,
            crash_reporting_enabled: false,
        };
        let result = init_observability(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_observability_config_tracing_levels() {
        let levels = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
        for level in levels {
            let config = ObservabilityConfig {
                tracing_level: level.to_string(),
                metrics_enabled: false,
                crash_reporting_enabled: false,
            };
            assert_eq!(config.tracing_level, level);
        }
    }

    #[test]
    fn test_observability_config_serialization() {
        let config = ObservabilityConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("INFO"));
        assert!(serialized.contains("metrics_enabled"));
    }

    #[test]
    fn test_observability_config_deserialization() {
        let json =
            r#"{"tracing_level":"DEBUG","metrics_enabled":false,"crash_reporting_enabled":true}"#;
        let config: ObservabilityConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.tracing_level, "DEBUG");
        assert!(!config.metrics_enabled);
        assert!(config.crash_reporting_enabled);
    }

    #[test]
    fn test_observability_config_clone() {
        let config = ObservabilityConfig {
            tracing_level: "TRACE".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: true,
        };
        let cloned = config.clone();
        assert_eq!(config.tracing_level, cloned.tracing_level);
        assert_eq!(config.metrics_enabled, cloned.metrics_enabled);
    }

    #[test]
    fn test_observability_config_debug() {
        let config = ObservabilityConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ObservabilityConfig"));
        assert!(debug_str.contains("INFO"));
    }

    #[test]
    fn test_observability_plugin_default() {
        let plugin = ObservabilityPlugin::default();
        assert_eq!(plugin.config.tracing_level, "INFO");
        assert!(plugin.config.metrics_enabled);
    }

    #[test]
    fn test_observability_plugin_build_with_metrics() {
        let mut app = App::new();
        let config = ObservabilityConfig {
            tracing_level: "INFO".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: true,
        };
        let plugin = ObservabilityPlugin::new(config);
        plugin.build(&mut app);

        let state = app.world.get_resource::<ObservabilityState>().unwrap();
        assert!(state.config.metrics_enabled);
    }

    #[test]
    fn test_observability_plugin_build_without_metrics() {
        let mut app = App::new();
        let config = ObservabilityConfig {
            tracing_level: "INFO".to_string(),
            metrics_enabled: false,
            crash_reporting_enabled: false,
        };
        let plugin = ObservabilityPlugin::new(config);
        plugin.build(&mut app);

        let state = app.world.get_resource::<ObservabilityState>().unwrap();
        assert!(!state.config.metrics_enabled);
    }

    #[test]
    fn test_observability_system_with_metrics() {
        let mut world = astraweave_ecs::World::default();
        let config = ObservabilityConfig {
            tracing_level: "INFO".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: false,
        };
        world.insert_resource(ObservabilityState::new(config));

        // Should not panic
        observability_system(&mut world);
    }

    #[test]
    fn test_observability_system_without_metrics() {
        let mut world = astraweave_ecs::World::default();
        let config = ObservabilityConfig {
            tracing_level: "INFO".to_string(),
            metrics_enabled: false,
            crash_reporting_enabled: false,
        };
        world.insert_resource(ObservabilityState::new(config));

        // Should not panic
        observability_system(&mut world);
    }

    #[test]
    fn test_observability_system_no_state() {
        let mut world = astraweave_ecs::World::default();
        // Should not panic even without state
        observability_system(&mut world);
    }

    #[test]
    fn test_trace_span_macro() {
        // Just verify macro compiles and runs
        let _span = trace_span!("test_span");
    }

    #[test]
    fn test_debug_span_macro() {
        let _span = debug_span!("debug_test");
    }

    #[test]
    fn test_info_span_macro() {
        let _span = info_span!("info_test");
    }

    #[test]
    fn test_warn_span_macro() {
        let _span = warn_span!("warn_test");
    }

    #[test]
    fn test_error_span_macro() {
        let _span = error_span!("error_test");
    }

    #[test]
    fn test_init_tracing_with_all_levels() {
        // Test all tracing level paths
        for level in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR", "UNKNOWN"] {
            let config = ObservabilityConfig {
                tracing_level: level.to_string(),
                metrics_enabled: false,
                crash_reporting_enabled: false,
            };
            // Just verify it doesn't panic
            let _ = init_observability(config);
        }
    }

    #[test]
    fn test_init_observability_full_stack() {
        let config = ObservabilityConfig {
            tracing_level: "TRACE".to_string(),
            metrics_enabled: true,
            crash_reporting_enabled: true,
        };
        let result = init_observability(config);
        assert!(result.is_ok());
    }
}
