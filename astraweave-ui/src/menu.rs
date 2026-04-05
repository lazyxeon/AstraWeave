/// Menu system for in-game menus (main menu, pause menu, settings)
///
/// This module provides a menu state machine and menu UI components.
use serde::{Deserialize, Serialize};

/// Menu states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MenuState {
    /// Main menu (game not started)
    MainMenu,
    /// Pause menu (game paused)
    PauseMenu,
    /// Settings menu
    SettingsMenu,
    /// No menu visible (in-game)
    None,
}

/// Graphics quality presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

impl QualityPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityPreset::Low => "Low",
            QualityPreset::Medium => "Medium",
            QualityPreset::High => "High",
            QualityPreset::Ultra => "Ultra",
        }
    }

    pub fn all() -> &'static [QualityPreset] {
        &[
            QualityPreset::Low,
            QualityPreset::Medium,
            QualityPreset::High,
            QualityPreset::Ultra,
        ]
    }
}

/// Graphics settings state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u32, u32),
    pub quality: QualityPreset,
    pub fullscreen: bool,
    pub vsync: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080),
            quality: QualityPreset::High,
            fullscreen: false,
            vsync: true,
        }
    }
}

/// Audio settings for volume and mute controls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Master volume (0-100)
    pub master_volume: f32,
    /// Music volume (0-100)
    pub music_volume: f32,
    /// SFX volume (0-100)
    pub sfx_volume: f32,
    /// Voice volume (0-100)
    pub voice_volume: f32,
    /// Master mute
    pub master_mute: bool,
    /// Music mute
    pub music_mute: bool,
    /// SFX mute
    pub sfx_mute: bool,
    /// Voice mute
    pub voice_mute: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            master_volume: 100.0,
            music_volume: 80.0,
            sfx_volume: 90.0,
            voice_volume: 100.0,
            master_mute: false,
            music_mute: false,
            sfx_mute: false,
            voice_mute: false,
        }
    }
}

/// Key bindings for game controls
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlsSettings {
    /// Move forward key
    pub move_forward: String,
    /// Move backward key
    pub move_backward: String,
    /// Move left key
    pub move_left: String,
    /// Move right key
    pub move_right: String,
    /// Jump key
    pub jump: String,
    /// Crouch key
    pub crouch: String,
    /// Sprint key
    pub sprint: String,
    /// Attack key
    pub attack: String,
    /// Interact key
    pub interact: String,
    /// Inventory key
    pub inventory: String,
    /// Mouse sensitivity (0.1 - 5.0x)
    pub mouse_sensitivity: f32,
    /// Invert Y-axis
    pub invert_y: bool,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        Self {
            move_forward: "W".to_string(),
            move_backward: "S".to_string(),
            move_left: "A".to_string(),
            move_right: "D".to_string(),
            jump: "Space".to_string(),
            crouch: "LControl".to_string(),
            sprint: "LShift".to_string(),
            attack: "Mouse0".to_string(),
            interact: "E".to_string(),
            inventory: "Tab".to_string(),
            mouse_sensitivity: 1.0,
            invert_y: false,
        }
    }
}

/// Settings state (holds all settings categories)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SettingsState {
    #[serde(default)]
    pub graphics: GraphicsSettings,
    #[serde(default)]
    pub audio: AudioSettings,
    #[serde(default)]
    pub controls: ControlsSettings,
}

/// Actions that can be triggered by menus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MenuAction {
    /// Start new game
    NewGame,
    /// Load saved game
    LoadGame,
    /// Save current game
    SaveGame,
    /// Resume from pause
    Resume,
    /// Open settings menu
    Settings,
    /// Apply settings (save to disk)
    ApplySettings,
    /// Cancel settings (revert changes)
    CancelSettings,
    /// Quit to main menu or exit
    Quit,
    /// No action
    None,
}

/// Menu manager - handles menu state transitions
pub struct MenuManager {
    state: MenuState,
    /// Track previous state for "Back" functionality
    previous_state: Option<MenuState>,
    /// Settings state (current + original for revert)
    pub settings: SettingsState,
    settings_original: SettingsState,
    /// Key rebinding state (which key is being rebound, if any)
    pub rebinding_key: Option<String>,
}

impl MenuManager {
    /// Create new menu manager (starts at main menu, loads saved settings)
    pub fn new() -> Self {
        let settings = crate::persistence::load_settings();
        Self {
            state: MenuState::MainMenu,
            previous_state: None,
            settings: settings.clone(),
            settings_original: settings,
            rebinding_key: None,
        }
    }

    /// Show the current menu and return any action taken
    pub fn show(&mut self, ctx: &egui::Context) -> MenuAction {
        match self.state {
            MenuState::MainMenu => crate::menus::show_main_menu(ctx),
            MenuState::PauseMenu => crate::menus::show_pause_menu(ctx),
            MenuState::SettingsMenu => {
                crate::menus::show_settings_menu(ctx, &mut self.settings, &mut self.rebinding_key)
            }
            MenuState::None => MenuAction::None,
        }
    }

    /// Handle a menu action (updates state)
    pub fn handle_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::NewGame => {
                self.previous_state = Some(self.state);
                self.state = MenuState::None; // Go to game
            }
            MenuAction::Resume => {
                self.previous_state = Some(self.state);
                self.state = MenuState::None; // Resume game
            }
            MenuAction::Settings => {
                self.previous_state = Some(self.state);
                self.state = MenuState::SettingsMenu;
            }
            MenuAction::Quit => {
                // Context-sensitive quit behavior
                match self.state {
                    MenuState::PauseMenu => {
                        // Quit from pause = go to main menu
                        self.previous_state = Some(self.state);
                        self.state = MenuState::MainMenu;
                    }
                    MenuState::SettingsMenu => {
                        // Back from settings = go to previous menu
                        if let Some(prev) = self.previous_state {
                            self.state = prev;
                            self.previous_state = None;
                        } else {
                            // Fallback: go to main menu
                            self.state = MenuState::MainMenu;
                        }
                    }
                    MenuState::MainMenu => {
                        // Quit from main menu = close application (handled by caller)
                    }
                    MenuState::None => {
                        // Should not happen (no menu visible)
                    }
                }
            }
            MenuAction::ApplySettings => {
                // Save settings to disk and update original state
                self.apply_settings();
            }
            MenuAction::CancelSettings => {
                // Revert to last saved settings
                self.revert_settings();
            }
            _ => {}
        }
    }

    /// Toggle pause menu (called on ESC key)
    pub fn toggle_pause(&mut self) {
        match self.state {
            MenuState::None => {
                self.previous_state = Some(self.state);
                self.state = MenuState::PauseMenu;
            }
            MenuState::PauseMenu => {
                self.previous_state = Some(self.state);
                self.state = MenuState::None;
            }
            MenuState::SettingsMenu => {
                // ESC from settings = go back to previous menu
                if let Some(prev) = self.previous_state {
                    self.state = prev;
                    self.previous_state = None;
                } else {
                    // Fallback: go to pause menu if in-game, otherwise main menu
                    self.state = MenuState::PauseMenu;
                }
            }
            _ => {}
        }
    }

    /// Check if any menu is currently visible
    pub fn is_menu_visible(&self) -> bool {
        self.state != MenuState::None
    }

    /// Check if we're on the main menu
    pub fn is_main_menu(&self) -> bool {
        self.state == MenuState::MainMenu
    }

    /// Get current menu state
    pub fn current_state(&self) -> MenuState {
        self.state
    }

    /// Apply settings changes (saves to disk and updates original)
    pub fn apply_settings(&mut self) {
        // Save to disk
        if let Err(e) = crate::persistence::save_settings(&self.settings) {
            log::error!("Failed to save settings: {}", e);
        }

        // Update original state
        self.settings_original = self.settings.clone();
        // In future: Apply settings to window/renderer here
    }

    /// Revert settings changes (restore from original, does not save)
    pub fn revert_settings(&mut self) {
        self.settings = self.settings_original.clone();
    }

    /// Check if settings have been modified
    pub fn settings_modified(&self) -> bool {
        self.settings != self.settings_original
    }

    /// Reset controls to default key bindings
    pub fn reset_controls_to_default(&mut self) {
        self.settings.controls = ControlsSettings::default();
    }
}

impl Default for MenuManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_preset_all() {
        let presets = QualityPreset::all();
        assert_eq!(presets.len(), 4);
        assert_eq!(presets[0], QualityPreset::Low);
        assert_eq!(presets[3], QualityPreset::Ultra);
    }

    #[test]
    fn test_quality_preset_as_str() {
        assert_eq!(QualityPreset::Low.as_str(), "Low");
        assert_eq!(QualityPreset::Medium.as_str(), "Medium");
        assert_eq!(QualityPreset::High.as_str(), "High");
        assert_eq!(QualityPreset::Ultra.as_str(), "Ultra");
    }

    #[test]
    fn test_graphics_settings_default() {
        let settings = GraphicsSettings::default();
        assert_eq!(settings.resolution, (1920, 1080));
        assert_eq!(settings.quality, QualityPreset::High);
        assert_eq!(settings.fullscreen, false);
        assert_eq!(settings.vsync, true);
    }

    #[test]
    fn test_audio_settings_default() {
        let settings = AudioSettings::default();
        assert_eq!(settings.master_volume, 100.0);
        assert_eq!(settings.music_volume, 80.0);
        assert_eq!(settings.sfx_volume, 90.0);
        assert_eq!(settings.voice_volume, 100.0);
        assert!(!settings.master_mute);
        assert!(!settings.music_mute);
        assert!(!settings.sfx_mute);
        assert!(!settings.voice_mute);
    }

    #[test]
    fn test_controls_settings_default() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.move_forward, "W");
        assert_eq!(settings.move_backward, "S");
        assert_eq!(settings.move_left, "A");
        assert_eq!(settings.move_right, "D");
        assert_eq!(settings.jump, "Space");
        assert_eq!(settings.crouch, "LControl");
        assert_eq!(settings.sprint, "LShift");
        assert_eq!(settings.attack, "Mouse0");
        assert_eq!(settings.interact, "E");
        assert_eq!(settings.inventory, "Tab");
        assert_eq!(settings.mouse_sensitivity, 1.0);
        assert!(!settings.invert_y);
    }

    #[test]
    fn test_menu_manager_new_starts_at_main_menu() {
        let manager = MenuManager::new();
        assert_eq!(manager.state, MenuState::MainMenu);
        assert_eq!(manager.previous_state, None);
        assert!(manager.is_main_menu());
        assert!(manager.is_menu_visible());
    }

    #[test]
    fn test_menu_manager_handle_new_game() {
        let mut manager = MenuManager::new();
        manager.handle_action(MenuAction::NewGame);
        assert_eq!(manager.state, MenuState::None);
        assert_eq!(manager.previous_state, Some(MenuState::MainMenu));
        assert!(!manager.is_menu_visible());
    }

    #[test]
    fn test_menu_manager_toggle_pause() {
        let mut manager = MenuManager::new();
        manager.state = MenuState::None; // In-game

        // Toggle pause on
        manager.toggle_pause();
        assert_eq!(manager.state, MenuState::PauseMenu);
        assert!(manager.is_menu_visible());

        // Toggle pause off
        manager.toggle_pause();
        assert_eq!(manager.state, MenuState::None);
        assert!(!manager.is_menu_visible());
    }

    #[test]
    fn test_menu_manager_settings_navigation() {
        let mut manager = MenuManager::new();
        assert_eq!(manager.state, MenuState::MainMenu);

        // Go to settings
        manager.handle_action(MenuAction::Settings);
        assert_eq!(manager.state, MenuState::SettingsMenu);
        assert_eq!(manager.previous_state, Some(MenuState::MainMenu));

        // Quit from settings (back to previous)
        manager.handle_action(MenuAction::Quit);
        assert_eq!(manager.state, MenuState::MainMenu);
    }

    #[test]
    fn test_menu_manager_quit_from_pause() {
        let mut manager = MenuManager::new();
        manager.state = MenuState::PauseMenu;

        // Quit from pause = go to main menu
        manager.handle_action(MenuAction::Quit);
        assert_eq!(manager.state, MenuState::MainMenu);
    }

    #[test]
    fn test_menu_manager_settings_modified() {
        let mut manager = MenuManager::new();
        assert!(!manager.settings_modified());

        // Modify graphics settings
        manager.settings.graphics.fullscreen = !manager.settings.graphics.fullscreen;
        assert!(manager.settings_modified());

        // Revert changes
        manager.revert_settings();
        assert!(!manager.settings_modified());
    }

    #[test]
    fn test_menu_manager_apply_settings() {
        let mut manager = MenuManager::new();

        // Use a value guaranteed different from whatever was loaded
        let orig = manager.settings.audio.master_volume;
        let different_vol = if (orig - 42.0).abs() < 0.01 {
            73.0
        } else {
            42.0
        };
        manager.settings.audio.master_volume = different_vol;
        assert!(manager.settings_modified());

        // Apply (saves and updates original)
        manager.apply_settings();
        assert!(!manager.settings_modified());
        assert_eq!(manager.settings_original.audio.master_volume, different_vol);
    }

    #[test]
    fn test_menu_manager_reset_controls() {
        let mut manager = MenuManager::new();

        // Modify controls
        manager.settings.controls.move_forward = "Y".to_string();
        assert_ne!(manager.settings.controls.move_forward, "W");

        // Reset to default
        manager.reset_controls_to_default();
        assert_eq!(manager.settings.controls.move_forward, "W");
    }

    #[test]
    fn test_menu_manager_rebinding_state() {
        let mut manager = MenuManager::new();
        assert_eq!(manager.rebinding_key, None);

        manager.rebinding_key = Some("move_forward".to_string());
        assert_eq!(manager.rebinding_key, Some("move_forward".to_string()));
    }

    #[test]
    fn test_menu_action_equality() {
        assert_eq!(MenuAction::NewGame, MenuAction::NewGame);
        assert_ne!(MenuAction::NewGame, MenuAction::LoadGame);
        assert_eq!(MenuAction::None, MenuAction::None);
    }

    #[test]
    fn test_menu_state_serialization() {
        // Test that MenuState can be serialized (required for save/load)
        let state = MenuState::SettingsMenu;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: MenuState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_settings_state_serialization() {
        let mut settings = SettingsState::default();
        settings.graphics.fullscreen = true;
        settings.audio.master_volume = 75.0;
        settings.controls.mouse_sensitivity = 1.5;

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: SettingsState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.graphics.fullscreen, true);
        assert_eq!(deserialized.audio.master_volume, 75.0);
        assert_eq!(deserialized.controls.mouse_sensitivity, 1.5);
    }
}
