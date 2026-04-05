//! Splash screen and loading screen system for game polish
//!
//! Provides configurable splash screens for branding and loading screens
//! with progress bars for asset loading.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Splash screen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplashScreen {
    /// Path to splash image (PNG, JPEG, or animated GIF)
    pub image_path: PathBuf,
    /// Duration to display (None = wait for input)
    pub duration: Option<Duration>,
    /// Fade in duration
    pub fade_in: Duration,
    /// Fade out duration
    pub fade_out: Duration,
    /// Background color (RGBA)
    pub background_color: [f32; 4],
    /// Allow skip with any key/click
    pub skippable: bool,
}

impl Default for SplashScreen {
    fn default() -> Self {
        Self {
            image_path: PathBuf::from("splash.png"),
            duration: Some(Duration::from_secs(3)),
            fade_in: Duration::from_millis(500),
            fade_out: Duration::from_millis(500),
            background_color: [0.0, 0.0, 0.0, 1.0],
            skippable: true,
        }
    }
}

/// Splash screen sequence for multiple screens
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SplashSequence {
    /// List of splash screens to show in order
    pub screens: Vec<SplashScreen>,
    /// Skip entire sequence on any input
    pub skip_all_on_input: bool,
}

impl SplashSequence {
    /// Create a new empty splash sequence
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a splash screen to the sequence
    pub fn add_screen(mut self, screen: SplashScreen) -> Self {
        self.screens.push(screen);
        self
    }

    /// Add an engine logo splash (AstraWeave branding)
    pub fn with_engine_logo(self) -> Self {
        self.add_screen(SplashScreen {
            image_path: PathBuf::from("engine_logo.png"),
            duration: Some(Duration::from_secs(2)),
            fade_in: Duration::from_millis(400),
            fade_out: Duration::from_millis(400),
            background_color: [0.05, 0.05, 0.08, 1.0],
            skippable: false,
        })
    }

    /// Add a publisher logo splash
    pub fn with_publisher_logo(self, logo_path: impl Into<PathBuf>) -> Self {
        self.add_screen(SplashScreen {
            image_path: logo_path.into(),
            duration: Some(Duration::from_secs(2)),
            fade_in: Duration::from_millis(500),
            fade_out: Duration::from_millis(500),
            background_color: [0.0, 0.0, 0.0, 1.0],
            skippable: true,
        })
    }

    /// Total duration of all splash screens
    pub fn total_duration(&self) -> Duration {
        self.screens
            .iter()
            .map(|s| {
                let display = s.duration.unwrap_or(Duration::ZERO);
                display + s.fade_in + s.fade_out
            })
            .sum()
    }
}

/// Loading screen style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LoadingStyle {
    /// Simple centered spinner
    Spinner,
    /// Horizontal progress bar at bottom
    ProgressBar,
    /// Full-screen background with progress bar
    FullScreen,
    /// Minimalist dot animation
    Dots,
    /// Custom artwork with tips
    ArtworkWithTips,
}

impl std::fmt::Display for LoadingStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadingStyle::Spinner => write!(f, "Spinner"),
            LoadingStyle::ProgressBar => write!(f, "Progress Bar"),
            LoadingStyle::FullScreen => write!(f, "Full Screen"),
            LoadingStyle::Dots => write!(f, "Dots"),
            LoadingStyle::ArtworkWithTips => write!(f, "Artwork With Tips"),
        }
    }
}

impl Default for LoadingStyle {
    fn default() -> Self {
        Self::ProgressBar
    }
}

impl LoadingStyle {
    /// Returns all loading styles.
    pub fn all() -> &'static [Self] {
        &[
            LoadingStyle::Spinner,
            LoadingStyle::ProgressBar,
            LoadingStyle::FullScreen,
            LoadingStyle::Dots,
            LoadingStyle::ArtworkWithTips,
        ]
    }

    /// Returns the display name of this style.
    pub fn name(&self) -> &'static str {
        match self {
            LoadingStyle::Spinner => "Spinner",
            LoadingStyle::ProgressBar => "Progress Bar",
            LoadingStyle::FullScreen => "Full Screen",
            LoadingStyle::Dots => "Dots",
            LoadingStyle::ArtworkWithTips => "Artwork With Tips",
        }
    }

    /// Returns an icon for this style.
    pub fn icon(&self) -> &'static str {
        match self {
            LoadingStyle::Spinner => "[Sync]",
            LoadingStyle::ProgressBar => "█",
            LoadingStyle::FullScreen => "[Img]",
            LoadingStyle::Dots => "•••",
            LoadingStyle::ArtworkWithTips => "[Art]",
        }
    }

    /// Returns true if this style shows a progress indicator.
    pub fn shows_progress(&self) -> bool {
        matches!(
            self,
            LoadingStyle::ProgressBar | LoadingStyle::FullScreen | LoadingStyle::ArtworkWithTips
        )
    }

    /// Returns true if this style is animated.
    pub fn is_animated(&self) -> bool {
        matches!(self, LoadingStyle::Spinner | LoadingStyle::Dots)
    }

    /// Returns true if this style supports background images.
    pub fn supports_background(&self) -> bool {
        matches!(
            self,
            LoadingStyle::FullScreen | LoadingStyle::ArtworkWithTips
        )
    }
}

/// Loading screen configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingScreen {
    /// Display style
    pub style: LoadingStyle,
    /// Background image (optional, for FullScreen/ArtworkWithTips)
    pub background_image: Option<PathBuf>,
    /// Background color
    pub background_color: [f32; 4],
    /// Progress bar color
    pub progress_color: [f32; 4],
    /// Show percentage text
    pub show_percentage: bool,
    /// Show current task description
    pub show_task_description: bool,
    /// Loading tips to cycle through
    pub tips: Vec<String>,
    /// Tip change interval
    pub tip_interval: Duration,
    /// Minimum display time (prevent flash)
    pub minimum_display_time: Duration,
}

impl Default for LoadingScreen {
    fn default() -> Self {
        Self {
            style: LoadingStyle::ProgressBar,
            background_image: None,
            background_color: [0.08, 0.08, 0.12, 1.0],
            progress_color: [0.3, 0.6, 1.0, 1.0],
            show_percentage: true,
            show_task_description: true,
            tips: vec![],
            tip_interval: Duration::from_secs(5),
            minimum_display_time: Duration::from_millis(500),
        }
    }
}

impl LoadingScreen {
    /// Create a loading screen with gaming tips
    pub fn with_tips(mut self, tips: Vec<String>) -> Self {
        self.tips = tips;
        self
    }

    /// Add a single tip
    pub fn add_tip(mut self, tip: impl Into<String>) -> Self {
        self.tips.push(tip.into());
        self
    }
}

/// Loading progress tracker
#[derive(Debug, Clone)]
pub struct LoadingProgress {
    /// Total number of tasks
    pub total_tasks: usize,
    /// Completed tasks
    pub completed_tasks: usize,
    /// Current task description
    pub current_task: String,
    /// Current tip index
    pub current_tip_index: usize,
    /// Start time
    pub started_at: std::time::Instant,
}

impl LoadingProgress {
    /// Create new loading progress
    pub fn new(total_tasks: usize) -> Self {
        Self {
            total_tasks,
            completed_tasks: 0,
            current_task: "Initializing...".to_string(),
            current_tip_index: 0,
            started_at: std::time::Instant::now(),
        }
    }

    /// Get progress percentage (0.0 - 1.0)
    pub fn percentage(&self) -> f32 {
        if self.total_tasks == 0 {
            return 1.0;
        }
        self.completed_tasks as f32 / self.total_tasks as f32
    }

    /// Update progress
    pub fn advance(&mut self, task_description: impl Into<String>) {
        self.completed_tasks += 1;
        self.current_task = task_description.into();
    }

    /// Check if loading is complete
    pub fn is_complete(&self) -> bool {
        self.completed_tasks >= self.total_tasks
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }
}

/// Game state save/load configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConfig {
    /// Save file extension
    pub extension: String,
    /// Save directory name (relative to game data)
    pub directory: String,
    /// Maximum autosave slots
    pub max_autosaves: usize,
    /// Autosave interval (None = disabled)
    pub autosave_interval: Option<Duration>,
    /// Compress save files
    pub compress: bool,
    /// Include screenshot in save
    pub include_screenshot: bool,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            extension: "sav".to_string(),
            directory: "saves".to_string(),
            max_autosaves: 3,
            autosave_interval: Some(Duration::from_secs(300)), // 5 minutes
            compress: true,
            include_screenshot: true,
        }
    }
}

/// Save file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    /// Save slot name
    pub name: String,
    /// Game version
    pub version: String,
    /// Timestamp (Unix seconds)
    pub timestamp: u64,
    /// Playtime in seconds
    pub playtime_seconds: u64,
    /// Chapter/level name
    pub location: String,
    /// Optional screenshot path
    pub screenshot: Option<PathBuf>,
    /// Is this an autosave
    pub is_autosave: bool,
}

impl SaveMetadata {
    /// Create new save metadata
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            playtime_seconds: 0,
            location: "Unknown".to_string(),
            screenshot: None,
            is_autosave: false,
        }
    }
}

/// Save game manager
pub struct SaveManager {
    config: SaveConfig,
    save_dir: PathBuf,
}

impl SaveManager {
    /// Create new save manager
    pub fn new(config: SaveConfig, game_data_dir: impl Into<PathBuf>) -> Self {
        let game_data = game_data_dir.into();
        let save_dir = game_data.join(&config.directory);
        Self { config, save_dir }
    }

    /// Ensure save directory exists
    pub fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.save_dir).context("Failed to create save directory")?;
        Ok(())
    }

    /// Sanitize a slot name to prevent path traversal
    fn sanitize_slot_name(name: &str) -> String {
        name.replace(['/', '\\'], "")
            .replace("..", "")
            .trim_matches('.')
            .to_string()
    }

    /// Get save file path for a slot
    pub fn save_path(&self, slot_name: &str) -> PathBuf {
        let safe_name = Self::sanitize_slot_name(slot_name);
        self.save_dir
            .join(format!("{}.{}", safe_name, self.config.extension))
    }

    /// Get metadata path for a save
    pub fn metadata_path(&self, slot_name: &str) -> PathBuf {
        let safe_name = Self::sanitize_slot_name(slot_name);
        self.save_dir.join(format!("{}.meta.json", safe_name))
    }

    /// Save game state
    pub fn save(&self, slot_name: &str, data: &[u8], metadata: &SaveMetadata) -> Result<PathBuf> {
        self.ensure_dir()?;

        let save_path = self.save_path(slot_name);
        let meta_path = self.metadata_path(slot_name);

        // Write save data (optionally compressed)
        if self.config.compress {
            let compressed = zstd::encode_all(data, 3).context("Failed to compress save data")?;
            std::fs::write(&save_path, compressed)?;
        } else {
            std::fs::write(&save_path, data)?;
        }

        // Write metadata
        let meta_json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&meta_path, meta_json)?;

        tracing::info!("Saved game to {}", save_path.display());
        Ok(save_path)
    }

    /// Load game state
    pub fn load(&self, slot_name: &str) -> Result<(Vec<u8>, SaveMetadata)> {
        let save_path = self.save_path(slot_name);
        let meta_path = self.metadata_path(slot_name);

        // Load metadata
        let meta_json =
            std::fs::read_to_string(&meta_path).context("Failed to read save metadata")?;
        let metadata: SaveMetadata =
            serde_json::from_str(&meta_json).context("Failed to parse save metadata")?;

        // Load save data
        let raw_data = std::fs::read(&save_path).context("Failed to read save file")?;

        let data = if self.config.compress {
            zstd::decode_all(raw_data.as_slice()).context("Failed to decompress save data")?
        } else {
            raw_data
        };

        tracing::info!("Loaded game from {}", save_path.display());
        Ok((data, metadata))
    }

    /// List all save files
    pub fn list_saves(&self) -> Result<Vec<SaveMetadata>> {
        let mut saves = Vec::new();

        if !self.save_dir.exists() {
            return Ok(saves);
        }

        for entry in std::fs::read_dir(&self.save_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false)
                && path.to_string_lossy().ends_with(".meta.json")
            {
                if let Ok(meta_json) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<SaveMetadata>(&meta_json) {
                        saves.push(metadata);
                    }
                }
            }
        }

        // Sort by timestamp descending
        saves.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(saves)
    }

    /// Delete a save
    pub fn delete(&self, slot_name: &str) -> Result<()> {
        let save_path = self.save_path(slot_name);
        let meta_path = self.metadata_path(slot_name);

        if save_path.exists() {
            std::fs::remove_file(&save_path)?;
        }
        if meta_path.exists() {
            std::fs::remove_file(&meta_path)?;
        }

        tracing::info!("Deleted save: {}", slot_name);
        Ok(())
    }

    /// Create autosave (rotates old autosaves)
    pub fn autosave(&self, data: &[u8], mut metadata: SaveMetadata) -> Result<PathBuf> {
        metadata.is_autosave = true;

        // Rotate autosaves
        for i in (1..self.config.max_autosaves).rev() {
            let old_name = format!("autosave_{}", i);
            let new_name = format!("autosave_{}", i + 1);

            let old_save = self.save_path(&old_name);
            let old_meta = self.metadata_path(&old_name);
            let new_save = self.save_path(&new_name);
            let new_meta = self.metadata_path(&new_name);

            if old_save.exists() {
                let _ = std::fs::rename(&old_save, &new_save);
            }
            if old_meta.exists() {
                let _ = std::fs::rename(&old_meta, &new_meta);
            }
        }

        // Delete oldest if exceeds max
        let oldest_name = format!("autosave_{}", self.config.max_autosaves + 1);
        let _ = self.delete(&oldest_name);

        // Create new autosave_1
        metadata.name = "Autosave".to_string();
        self.save("autosave_1", data, &metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splash_sequence() {
        let seq = SplashSequence::new()
            .with_engine_logo()
            .with_publisher_logo("publisher.png");

        assert_eq!(seq.screens.len(), 2);
        assert!(seq.total_duration() > Duration::ZERO);
    }

    #[test]
    fn test_loading_progress() {
        let mut progress = LoadingProgress::new(10);
        assert_eq!(progress.percentage(), 0.0);

        progress.advance("Loading textures");
        assert_eq!(progress.percentage(), 0.1);
        assert_eq!(progress.completed_tasks, 1);

        for _ in 0..9 {
            progress.advance("Loading...");
        }
        assert!(progress.is_complete());
    }

    #[test]
    fn test_save_metadata() {
        let meta = SaveMetadata::new("Test Save", "1.0.0");
        assert_eq!(meta.name, "Test Save");
        assert!(meta.timestamp > 0);
        assert!(!meta.is_autosave);
    }

    #[test]
    fn test_save_config_default() {
        let config = SaveConfig::default();
        assert_eq!(config.extension, "sav");
        assert!(config.compress);
        assert_eq!(config.max_autosaves, 3);
    }

    #[test]
    fn test_loading_screen_with_tips() {
        let screen = LoadingScreen::default()
            .add_tip("Quick save with F5")
            .add_tip("Hold Shift to run faster");

        assert_eq!(screen.tips.len(), 2);
        assert!(screen.show_percentage);
    }

    // === LoadingStyle Display & helper tests ===

    #[test]
    fn test_loading_style_display() {
        assert_eq!(format!("{}", LoadingStyle::Spinner), "Spinner");
        assert_eq!(format!("{}", LoadingStyle::ProgressBar), "Progress Bar");
        assert_eq!(format!("{}", LoadingStyle::FullScreen), "Full Screen");
        assert_eq!(
            format!("{}", LoadingStyle::ArtworkWithTips),
            "Artwork With Tips"
        );
    }

    #[test]
    fn test_loading_style_all() {
        let all = LoadingStyle::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&LoadingStyle::Spinner));
        assert!(all.contains(&LoadingStyle::Dots));
    }

    #[test]
    fn test_loading_style_default() {
        assert_eq!(LoadingStyle::default(), LoadingStyle::ProgressBar);
    }

    #[test]
    fn test_loading_style_helpers() {
        assert!(LoadingStyle::ProgressBar.shows_progress());
        assert!(LoadingStyle::FullScreen.shows_progress());
        assert!(!LoadingStyle::Spinner.shows_progress());

        assert!(LoadingStyle::Spinner.is_animated());
        assert!(LoadingStyle::Dots.is_animated());
        assert!(!LoadingStyle::FullScreen.is_animated());

        assert!(LoadingStyle::FullScreen.supports_background());
        assert!(LoadingStyle::ArtworkWithTips.supports_background());
        assert!(!LoadingStyle::Spinner.supports_background());
    }

    #[test]
    fn test_loading_style_name_and_icon() {
        assert_eq!(LoadingStyle::Spinner.name(), "Spinner");
        assert_eq!(LoadingStyle::Spinner.icon(), "[Sync]");
        assert_eq!(LoadingStyle::ArtworkWithTips.icon(), "[Art]");
    }
}
