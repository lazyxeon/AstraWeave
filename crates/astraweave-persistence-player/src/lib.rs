#![forbid(unsafe_code)]
//! Player profile and progression persistence
//!
//! This crate provides a `PlayerProfile` system for saving/loading player settings,
//! stats, unlocks, and progression across game sessions.
//!
//! # Features
//!
//! - **TOML Persistence**: Human-readable format for user editing
//! - **Settings Management**: Graphics, audio, controls with live application
//! - **Progression Tracking**: Unlocks, achievements, stats
//! - **Autosave**: Periodic saving with configurable intervals
//! - **Corruption Recovery**: Resets to default on corrupted files
//!
//! # Quick Start
//!
//! ```no_run
//! use astraweave_persistence_player::PlayerProfile;
//!
//! // Load or create profile
//! let mut profile = PlayerProfile::quick_load().unwrap();
//!
//! // Make changes
//! profile.unlock_ability("Dash");
//! profile.grant_achievement("First Blood");
//! profile.record_kill();
//!
//! // Save
//! profile.quick_save().unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod utc_stamp;
pub use utc_stamp::format_utc_seconds;

mod autosave;
mod progression;
mod save_slots;
mod settings;

pub use autosave::*;
pub use save_slots::*;

/// Player profile containing settings, stats, unlocks, and progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    /// Version for future migrations
    pub version: u32,

    /// Player name
    pub name: String,

    /// Game settings (graphics, audio, controls)
    pub settings: GameSettings,

    /// Player statistics
    pub stats: PlayerStats,

    /// Unlocked content
    pub unlocks: Unlocks,

    /// Inventory (optional for now)
    #[serde(default)]
    pub inventory: Inventory,

    /// Quest progress (optional for now)
    #[serde(default)]
    pub quest_progress: QuestProgress,
}

/// Game settings (graphics, audio, controls)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameSettings {
    pub graphics: GraphicsSettings,
    pub audio: AudioSettings,
    pub controls: ControlSettings,
}

/// Graphics settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsSettings {
    /// Resolution (width × height)
    pub resolution: (u32, u32),

    /// Quality preset
    pub quality: QualityPreset,

    /// VSync enabled
    pub vsync: bool,

    /// Fullscreen mode
    pub fullscreen: bool,
}

/// Quality preset
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

/// Audio settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Master volume (0.0 - 1.0)
    pub master_volume: f32,

    /// Music volume (0.0 - 1.0)
    pub music_volume: f32,

    /// SFX volume (0.0 - 1.0)
    pub sfx_volume: f32,

    /// Voice volume (0.0 - 1.0)
    pub voice_volume: f32,

    /// Muted (overrides all volumes)
    pub muted: bool,
}

/// Control settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlSettings {
    /// Mouse sensitivity (0.1 - 2.0)
    pub mouse_sensitivity: f32,

    /// Invert Y axis
    pub invert_y: bool,

    /// Key bindings (action name → key)
    pub key_bindings: HashMap<String, String>,
}

/// Player statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    /// Total playtime (seconds)
    pub playtime_seconds: u64,

    /// Enemies defeated
    pub enemies_defeated: u32,

    /// Deaths
    pub deaths: u32,

    /// Achievements unlocked
    pub achievements: Vec<String>,
}

/// Unlocked content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Unlocks {
    /// Unlocked abilities
    pub abilities: Vec<String>,

    /// Unlocked items
    pub items: Vec<String>,

    /// Unlocked levels
    pub levels: Vec<String>,
}

/// Inventory (placeholder for now)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    // Will be implemented in Week 3+
}

/// Quest progress (placeholder for now)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestProgress {
    // Will be implemented in Week 3+
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for PlayerProfile {
    fn default() -> Self {
        Self {
            version: 1,
            name: "Player".to_string(),
            settings: GameSettings::default(),
            stats: PlayerStats::default(),
            unlocks: Unlocks::default(),
            inventory: Inventory::default(),
            quest_progress: QuestProgress::default(),
        }
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080),
            quality: QualityPreset::High,
            vsync: true,
            fullscreen: false,
        }
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 0.7,
            music_volume: 0.5,
            sfx_volume: 0.7,
            voice_volume: 0.8,
            muted: false,
        }
    }
}

impl Default for ControlSettings {
    fn default() -> Self {
        let mut key_bindings = HashMap::new();
        key_bindings.insert("forward".to_string(), "W".to_string());
        key_bindings.insert("backward".to_string(), "S".to_string());
        key_bindings.insert("left".to_string(), "A".to_string());
        key_bindings.insert("right".to_string(), "D".to_string());
        key_bindings.insert("jump".to_string(), "Space".to_string());
        key_bindings.insert("interact".to_string(), "E".to_string());

        Self {
            mouse_sensitivity: 1.0,
            invert_y: false,
            key_bindings,
        }
    }
}

// ============================================================================
// Save/Load API
// ============================================================================

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

impl PlayerProfile {
    /// Save profile to TOML file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::PlayerProfile;
    /// let profile = PlayerProfile::default();
    /// profile.save_to_file("saves/player_profile.toml").unwrap();
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_string =
            toml::to_string_pretty(self).context("Failed to serialize PlayerProfile to TOML")?;

        // Create parent directory if needed
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).context("Failed to create saves directory")?;
        }

        fs::write(&path, toml_string).context("Failed to write PlayerProfile to disk")?;

        Ok(())
    }

    /// Load profile from TOML file, or create default if missing/corrupted
    ///
    /// # Corruption Recovery
    ///
    /// If the file is corrupted (invalid TOML), this function will:
    /// 1. Log a warning to stderr
    /// 2. Return a default profile (not an error)
    ///
    /// This ensures the game always has a valid profile to work with.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::PlayerProfile;
    /// let profile = PlayerProfile::load_from_file("saves/player_profile.toml").unwrap();
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();

        // If file doesn't exist, return default
        if !path_ref.exists() {
            eprintln!("⚠️  Profile not found at {:?}, creating default", path_ref);
            return Ok(Self::default());
        }

        // Try to load file
        let contents = fs::read_to_string(path_ref).context("Failed to read PlayerProfile file")?;

        // Try to deserialize
        match toml::from_str::<Self>(&contents) {
            Ok(profile) => Ok(profile),
            Err(e) => {
                eprintln!("⚠️  Corrupted profile at {:?}: {}", path_ref, e);
                eprintln!("⚠️  Resetting to default profile");
                Ok(Self::default())
            }
        }
    }

    /// Get default save path
    pub fn default_path() -> PathBuf {
        PathBuf::from("saves/player_profile.toml")
    }

    /// Quick save (to default path)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::PlayerProfile;
    /// let profile = PlayerProfile::default();
    /// profile.quick_save().unwrap();
    /// ```
    pub fn quick_save(&self) -> Result<()> {
        self.save_to_file(Self::default_path())
    }

    /// Quick load (from default path)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use astraweave_persistence_player::PlayerProfile;
    /// let profile = PlayerProfile::quick_load().unwrap();
    /// ```
    pub fn quick_load() -> Result<Self> {
        Self::load_from_file(Self::default_path())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let profile = PlayerProfile::default();
        assert_eq!(profile.version, 1);
        assert_eq!(profile.name, "Player");
        assert_eq!(profile.settings.graphics.resolution, (1920, 1080));
        assert_eq!(profile.settings.audio.master_volume, 0.7);
    }

    #[test]
    fn test_roundtrip() {
        let profile = PlayerProfile::default();
        let path = "test_profile.toml";

        // Save
        profile.save_to_file(path).unwrap();

        // Load
        let loaded = PlayerProfile::load_from_file(path).unwrap();

        // Verify
        assert_eq!(profile.version, loaded.version);
        assert_eq!(profile.name, loaded.name);
        assert_eq!(
            profile.settings.graphics.resolution,
            loaded.settings.graphics.resolution
        );

        // Cleanup
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_corrupted_profile() {
        let path = "test_corrupted.toml";

        // Write invalid TOML
        std::fs::write(path, "invalid toml :::").unwrap();

        // Load should return default (not crash)
        let profile = PlayerProfile::load_from_file(path).unwrap();
        assert_eq!(profile.version, 1);

        // Cleanup
        std::fs::remove_file(path).unwrap();
    }
}
