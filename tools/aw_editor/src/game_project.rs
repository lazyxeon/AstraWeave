//! Game Project Configuration
//!
//! This module defines the game project manifest structure that describes
//! a complete game project. The manifest is stored as `game.toml` in the
//! project root and contains all metadata needed for building and shipping.
//!
//! # Example game.toml
//!
//! ```toml
//! [project]
//! name = "My Awesome Game"
//! version = "1.0.0"
//! author = "Game Studio"
//! description = "An epic adventure game"
//!
//! [build]
//! entry_scene = "scenes/main_menu.scene"
//! default_target = "windows"
//!
//! [assets]
//! include = ["assets/**/*", "content/**/*"]
//! exclude = ["**/*.psd", "**/*.blend"]
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Game project manifest - the central configuration for a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProject {
    /// Project metadata
    pub project: ProjectMetadata,
    /// Build configuration
    pub build: BuildSettings,
    /// Asset configuration
    #[serde(default)]
    pub assets: AssetSettings,
    /// Platform-specific overrides
    #[serde(default)]
    pub platforms: PlatformOverrides,
}

/// Project metadata (name, version, author, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Game name (displayed in window title, store pages)
    pub name: String,
    /// Semantic version (e.g., "1.0.0")
    pub version: String,
    /// Author or studio name
    #[serde(default)]
    pub author: String,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Icon path (relative to project root)
    #[serde(default)]
    pub icon: Option<PathBuf>,
    /// Unique identifier for the game (e.g., com.studio.gamename)
    #[serde(default)]
    pub identifier: Option<String>,
}

/// Build configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSettings {
    /// Entry scene to load on game start
    pub entry_scene: PathBuf,
    /// Default build target platform
    #[serde(default = "default_target")]
    pub default_target: String,
    /// Default build profile
    #[serde(default = "default_profile")]
    pub default_profile: String,
    /// Output directory for builds
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    /// Additional cargo features to enable
    #[serde(default)]
    pub features: Vec<String>,
}

fn default_target() -> String {
    "windows".to_string()
}

fn default_profile() -> String {
    "release".to_string()
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("builds")
}

/// Asset packaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetSettings {
    /// Glob patterns for assets to include
    #[serde(default = "default_include_patterns")]
    pub include: Vec<String>,
    /// Glob patterns for assets to exclude
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Enable asset compression
    #[serde(default = "default_true")]
    pub compress: bool,
    /// Compression level (1-22 for zstd)
    #[serde(default = "default_compression_level")]
    pub compression_level: u8,
}

fn default_include_patterns() -> Vec<String> {
    vec![
        "assets/**/*".to_string(),
        "content/**/*".to_string(),
        "scenes/**/*".to_string(),
    ]
}

fn default_true() -> bool {
    true
}

fn default_compression_level() -> u8 {
    3
}

/// Platform-specific build overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformOverrides {
    #[serde(default)]
    pub windows: Option<PlatformConfig>,
    #[serde(default)]
    pub linux: Option<PlatformConfig>,
    #[serde(default)]
    pub macos: Option<PlatformConfig>,
    #[serde(default)]
    pub web: Option<PlatformConfig>,
}

/// Configuration for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformConfig {
    /// Platform-specific icon
    pub icon: Option<PathBuf>,
    /// Additional features for this platform
    #[serde(default)]
    pub features: Vec<String>,
    /// Platform-specific asset overrides
    #[serde(default)]
    pub asset_overrides: Vec<AssetOverride>,
}

/// Override for specific assets on a platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetOverride {
    /// Original asset path pattern
    pub pattern: String,
    /// Replacement path or processing action
    pub replacement: String,
}

impl GameProject {
    /// Load a game project from a TOML file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GameProjectError> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| GameProjectError::Io(e.to_string()))?;

        toml::from_str(&content).map_err(|e| GameProjectError::Parse(e.to_string()))
    }

    /// Save the game project to a TOML file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), GameProjectError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| GameProjectError::Serialize(e.to_string()))?;

        let tmp_path = path.as_ref().with_extension("tmp");
        std::fs::write(&tmp_path, &content)
            .and_then(|_| std::fs::rename(&tmp_path, path.as_ref()))
            .map_err(|e| GameProjectError::Io(e.to_string()))
    }

    /// Create a new game project with default settings
    pub fn new(name: &str, entry_scene: &str) -> Self {
        Self {
            project: ProjectMetadata {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                author: String::new(),
                description: String::new(),
                icon: None,
                identifier: None,
            },
            build: BuildSettings {
                entry_scene: PathBuf::from(entry_scene),
                default_target: default_target(),
                default_profile: default_profile(),
                output_dir: default_output_dir(),
                features: Vec::new(),
            },
            assets: AssetSettings::default(),
            platforms: PlatformOverrides::default(),
        }
    }

    /// Get the game name
    pub fn name(&self) -> &str {
        &self.project.name
    }

    /// Get the game version
    pub fn version(&self) -> &str {
        &self.project.version
    }

    /// Check if the project has an icon configured
    pub fn has_icon(&self) -> bool {
        self.project.icon.is_some()
    }

    /// Check if the project has a unique identifier
    pub fn has_identifier(&self) -> bool {
        self.project.identifier.is_some()
    }

    /// Get the number of enabled features
    pub fn feature_count(&self) -> usize {
        self.build.features.len()
    }

    /// Get a summary of the project
    pub fn summary(&self) -> String {
        format!(
            "{} v{} ({})",
            self.project.name, self.project.version, self.build.default_target
        )
    }

    /// Check if project targets a specific platform
    pub fn targets_platform(&self, platform: &str) -> bool {
        self.build.default_target == platform
    }

    /// Check if project has platform-specific config
    pub fn has_platform_config(&self, platform: &str) -> bool {
        match platform {
            "windows" => self.platforms.windows.is_some(),
            "linux" => self.platforms.linux.is_some(),
            "macos" => self.platforms.macos.is_some(),
            "web" => self.platforms.web.is_some(),
            _ => false,
        }
    }

    /// Find game.toml in the current directory or parent directories
    pub fn find_project_file() -> Option<PathBuf> {
        let mut current = std::env::current_dir().ok()?;

        loop {
            let manifest = current.join("game.toml");
            if manifest.exists() {
                return Some(manifest);
            }

            if !current.pop() {
                break;
            }
        }

        None
    }

    /// Get the project root directory (parent of game.toml)
    pub fn project_root(&self, manifest_path: &Path) -> PathBuf {
        manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Validate the project configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.project.name.is_empty() {
            errors.push("Project name is required".to_string());
        }

        if self.build.entry_scene.as_os_str().is_empty() {
            errors.push("Entry scene is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for GameProject {
    fn default() -> Self {
        Self::new("Untitled Game", "scenes/main.scene")
    }
}

impl Default for AssetSettings {
    fn default() -> Self {
        Self {
            include: default_include_patterns(),
            exclude: Vec::new(),
            compress: true,
            compression_level: default_compression_level(),
        }
    }
}

impl AssetSettings {
    /// Get total pattern count (include + exclude)
    pub fn pattern_count(&self) -> usize {
        self.include.len() + self.exclude.len()
    }

    /// Check if compression is enabled with high level (>= 10)
    pub fn has_high_compression(&self) -> bool {
        self.compress && self.compression_level >= 10
    }

    /// Check if there are any exclude patterns
    pub fn has_excludes(&self) -> bool {
        !self.exclude.is_empty()
    }

    /// Get compression summary
    pub fn compression_summary(&self) -> String {
        if self.compress {
            format!("Enabled (level {})", self.compression_level)
        } else {
            "Disabled".to_string()
        }
    }
}

/// Errors that can occur when working with game projects
#[derive(Debug, Clone)]
#[non_exhaustive]
#[must_use]
pub enum GameProjectError {
    Io(String),
    Parse(String),
    Serialize(String),
    Validation(Vec<String>),
}

impl std::fmt::Display for GameProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Parse(e) => write!(f, "Parse error: {}", e),
            Self::Serialize(e) => write!(f, "Serialize error: {}", e),
            Self::Validation(errors) => write!(f, "Validation errors: {:?}", errors),
        }
    }
}

impl std::error::Error for GameProjectError {}

impl GameProjectError {
    /// Get the error category
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io(_) => "IO",
            Self::Parse(_) => "Parse",
            Self::Serialize(_) => "Serialize",
            Self::Validation(_) => "Validation",
        }
    }

    /// Check if this is an IO error
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_))
    }

    /// Check if this is a parse error
    pub fn is_parse(&self) -> bool {
        matches!(self, Self::Parse(_))
    }

    /// Check if this is a validation error
    pub fn is_validation(&self) -> bool {
        matches!(self, Self::Validation(_))
    }

    /// Get the error message
    pub fn message(&self) -> String {
        match self {
            Self::Io(e) => e.clone(),
            Self::Parse(e) => e.clone(),
            Self::Serialize(e) => e.clone(),
            Self::Validation(errors) => errors.join(", "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_project() {
        let project = GameProject::default();
        assert_eq!(project.project.name, "Untitled Game");
        assert_eq!(
            project.build.entry_scene,
            PathBuf::from("scenes/main.scene")
        );
    }

    #[test]
    fn test_new_project() {
        let project = GameProject::new("My Game", "levels/intro.scene");
        assert_eq!(project.project.name, "My Game");
        assert_eq!(
            project.build.entry_scene,
            PathBuf::from("levels/intro.scene")
        );
    }

    #[test]
    fn test_validate_empty_name() {
        let mut project = GameProject::default();
        project.project.name = String::new();

        let result = project.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains(&"Project name is required".to_string()));
    }

    #[test]
    fn test_serialize_deserialize() {
        let project = GameProject::new("Test Game", "test.scene");
        let toml = toml::to_string(&project).unwrap();
        let parsed: GameProject = toml::from_str(&toml).unwrap();

        assert_eq!(parsed.project.name, project.project.name);
        assert_eq!(parsed.build.entry_scene, project.build.entry_scene);
    }

    #[test]
    fn test_default_asset_settings() {
        let settings = AssetSettings::default();
        assert!(settings.compress);
        assert_eq!(settings.compression_level, 3);
        assert!(!settings.include.is_empty());
    }

    // ====================================================================
    // GameProject New Methods Tests
    // ====================================================================

    #[test]
    fn test_game_project_name() {
        let project = GameProject::new("Test Game", "main.scene");
        assert_eq!(project.name(), "Test Game");
    }

    #[test]
    fn test_game_project_version() {
        let project = GameProject::new("Test Game", "main.scene");
        assert_eq!(project.version(), "0.1.0");
    }

    #[test]
    fn test_game_project_has_icon() {
        let project = GameProject::default();
        assert!(!project.has_icon());
    }

    #[test]
    fn test_game_project_has_identifier() {
        let project = GameProject::default();
        assert!(!project.has_identifier());
    }

    #[test]
    fn test_game_project_feature_count() {
        let project = GameProject::default();
        assert_eq!(project.feature_count(), 0);
    }

    #[test]
    fn test_game_project_summary() {
        let project = GameProject::new("My Game", "main.scene");
        let summary = project.summary();
        assert!(summary.contains("My Game"));
        assert!(summary.contains("0.1.0"));
    }

    #[test]
    fn test_game_project_targets_platform() {
        let project = GameProject::default();
        assert!(project.targets_platform("windows"));
        assert!(!project.targets_platform("linux"));
    }

    #[test]
    fn test_game_project_has_platform_config() {
        let project = GameProject::default();
        assert!(!project.has_platform_config("windows"));
        assert!(!project.has_platform_config("unknown"));
    }

    // ====================================================================
    // AssetSettings New Methods Tests
    // ====================================================================

    #[test]
    fn test_asset_settings_pattern_count() {
        let settings = AssetSettings::default();
        assert!(settings.pattern_count() >= 3);
    }

    #[test]
    fn test_asset_settings_has_high_compression() {
        let mut settings = AssetSettings::default();
        assert!(!settings.has_high_compression());
        settings.compression_level = 15;
        assert!(settings.has_high_compression());
    }

    #[test]
    fn test_asset_settings_has_excludes() {
        let mut settings = AssetSettings::default();
        assert!(!settings.has_excludes());
        settings.exclude.push("*.tmp".to_string());
        assert!(settings.has_excludes());
    }

    #[test]
    fn test_asset_settings_compression_summary() {
        let settings = AssetSettings::default();
        let summary = settings.compression_summary();
        assert!(summary.contains("Enabled"));
        assert!(summary.contains("3"));
    }

    #[test]
    fn test_asset_settings_compression_summary_disabled() {
        let settings = AssetSettings {
            compress: false,
            ..Default::default()
        };
        assert_eq!(settings.compression_summary(), "Disabled");
    }

    // ====================================================================
    // GameProjectError New Methods Tests
    // ====================================================================

    #[test]
    fn test_game_project_error_category() {
        let io_err = GameProjectError::Io("test".to_string());
        assert_eq!(io_err.category(), "IO");

        let parse_err = GameProjectError::Parse("test".to_string());
        assert_eq!(parse_err.category(), "Parse");
    }

    #[test]
    fn test_game_project_error_is_io() {
        let err = GameProjectError::Io("test".to_string());
        assert!(err.is_io());
        assert!(!err.is_parse());
    }

    #[test]
    fn test_game_project_error_is_parse() {
        let err = GameProjectError::Parse("test".to_string());
        assert!(err.is_parse());
        assert!(!err.is_io());
    }

    #[test]
    fn test_game_project_error_is_validation() {
        let err = GameProjectError::Validation(vec!["error".to_string()]);
        assert!(err.is_validation());
    }

    #[test]
    fn test_game_project_error_message() {
        let err = GameProjectError::Io("file not found".to_string());
        assert_eq!(err.message(), "file not found");

        let val_err = GameProjectError::Validation(vec!["a".to_string(), "b".to_string()]);
        assert!(val_err.message().contains("a"));
        assert!(val_err.message().contains("b"));
    }
}
