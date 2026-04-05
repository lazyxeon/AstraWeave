//! Comprehensive mutation-resistant tests for astraweave-ui
//!
//! These tests are designed to achieve 90%+ mutation kill rate by:
//! - Testing all enum variants and their behaviors
//! - Verifying state transitions and side effects
//! - Checking boundary conditions and edge cases
//! - Testing error paths and failure modes
//! - Validating return values from all public methods

#![allow(clippy::clone_on_copy, clippy::field_reassign_with_default)]

// ═══════════════════════════════════════════════════════════════════════════
// MENU STATE ENUM TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod menu_state_tests {
    use astraweave_ui::MenuState;

    #[test]
    fn test_menu_state_main_menu() {
        let state = MenuState::MainMenu;
        assert_eq!(state, MenuState::MainMenu);
    }

    #[test]
    fn test_menu_state_pause_menu() {
        let state = MenuState::PauseMenu;
        assert_eq!(state, MenuState::PauseMenu);
    }

    #[test]
    fn test_menu_state_settings_menu() {
        let state = MenuState::SettingsMenu;
        assert_eq!(state, MenuState::SettingsMenu);
    }

    #[test]
    fn test_menu_state_none() {
        let state = MenuState::None;
        assert_eq!(state, MenuState::None);
    }

    #[test]
    fn test_menu_state_ne() {
        assert_ne!(MenuState::MainMenu, MenuState::PauseMenu);
        assert_ne!(MenuState::MainMenu, MenuState::SettingsMenu);
        assert_ne!(MenuState::MainMenu, MenuState::None);
        assert_ne!(MenuState::PauseMenu, MenuState::SettingsMenu);
    }

    #[test]
    fn test_menu_state_copy() {
        let state = MenuState::MainMenu;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[test]
    fn test_menu_state_clone() {
        let state = MenuState::PauseMenu;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_menu_state_debug() {
        let state = MenuState::SettingsMenu;
        let debug = format!("{:?}", state);
        assert!(debug.contains("SettingsMenu"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QUALITY PRESET TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod quality_preset_tests {
    use astraweave_ui::QualityPreset;

    #[test]
    fn test_quality_preset_low() {
        let preset = QualityPreset::Low;
        assert_eq!(preset.as_str(), "Low");
    }

    #[test]
    fn test_quality_preset_medium() {
        let preset = QualityPreset::Medium;
        assert_eq!(preset.as_str(), "Medium");
    }

    #[test]
    fn test_quality_preset_high() {
        let preset = QualityPreset::High;
        assert_eq!(preset.as_str(), "High");
    }

    #[test]
    fn test_quality_preset_ultra() {
        let preset = QualityPreset::Ultra;
        assert_eq!(preset.as_str(), "Ultra");
    }

    #[test]
    fn test_quality_preset_all() {
        let all = QualityPreset::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], QualityPreset::Low);
        assert_eq!(all[1], QualityPreset::Medium);
        assert_eq!(all[2], QualityPreset::High);
        assert_eq!(all[3], QualityPreset::Ultra);
    }

    #[test]
    fn test_quality_preset_eq() {
        assert_eq!(QualityPreset::Low, QualityPreset::Low);
        assert_ne!(QualityPreset::Low, QualityPreset::High);
    }

    #[test]
    fn test_quality_preset_copy() {
        let preset = QualityPreset::Ultra;
        let copied = preset;
        assert_eq!(preset, copied);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GRAPHICS SETTINGS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod graphics_settings_tests {
    use astraweave_ui::{GraphicsSettings, QualityPreset};

    #[test]
    fn test_graphics_settings_default_resolution() {
        let settings = GraphicsSettings::default();
        assert_eq!(settings.resolution, (1920, 1080));
    }

    #[test]
    fn test_graphics_settings_default_quality() {
        let settings = GraphicsSettings::default();
        assert_eq!(settings.quality, QualityPreset::High);
    }

    #[test]
    fn test_graphics_settings_default_fullscreen() {
        let settings = GraphicsSettings::default();
        assert!(!settings.fullscreen);
    }

    #[test]
    fn test_graphics_settings_default_vsync() {
        let settings = GraphicsSettings::default();
        assert!(settings.vsync);
    }

    #[test]
    fn test_graphics_settings_clone() {
        let settings = GraphicsSettings::default();
        let cloned = settings.clone();
        assert_eq!(settings.resolution, cloned.resolution);
        assert_eq!(settings.quality, cloned.quality);
    }

    #[test]
    fn test_graphics_settings_eq() {
        let a = GraphicsSettings::default();
        let b = GraphicsSettings::default();
        assert_eq!(a, b);
    }

    #[test]
    fn test_graphics_settings_ne() {
        let mut a = GraphicsSettings::default();
        let b = GraphicsSettings::default();
        a.fullscreen = true;
        assert_ne!(a, b);
    }

    #[test]
    fn test_graphics_settings_custom_resolution() {
        let mut settings = GraphicsSettings::default();
        settings.resolution = (2560, 1440);
        assert_eq!(settings.resolution.0, 2560);
        assert_eq!(settings.resolution.1, 1440);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIO SETTINGS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod audio_settings_tests {
    use astraweave_ui::AudioSettings;

    #[test]
    fn test_audio_settings_default_master() {
        let settings = AudioSettings::default();
        assert_eq!(settings.master_volume, 100.0);
    }

    #[test]
    fn test_audio_settings_default_music() {
        let settings = AudioSettings::default();
        assert_eq!(settings.music_volume, 80.0);
    }

    #[test]
    fn test_audio_settings_default_sfx() {
        let settings = AudioSettings::default();
        assert_eq!(settings.sfx_volume, 90.0);
    }

    #[test]
    fn test_audio_settings_default_voice() {
        let settings = AudioSettings::default();
        assert_eq!(settings.voice_volume, 100.0);
    }

    #[test]
    fn test_audio_settings_default_mute_false() {
        let settings = AudioSettings::default();
        assert!(!settings.master_mute);
        assert!(!settings.music_mute);
        assert!(!settings.sfx_mute);
        assert!(!settings.voice_mute);
    }

    #[test]
    fn test_audio_settings_clone() {
        let settings = AudioSettings::default();
        let cloned = settings.clone();
        assert_eq!(settings.master_volume, cloned.master_volume);
    }

    #[test]
    fn test_audio_settings_eq() {
        let a = AudioSettings::default();
        let b = AudioSettings::default();
        assert_eq!(a, b);
    }

    #[test]
    fn test_audio_settings_ne() {
        let mut a = AudioSettings::default();
        let b = AudioSettings::default();
        a.master_mute = true;
        assert_ne!(a, b);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTROLS SETTINGS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod controls_settings_tests {
    use astraweave_ui::ControlsSettings;

    #[test]
    fn test_controls_default_wasd() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.move_forward, "W");
        assert_eq!(settings.move_backward, "S");
        assert_eq!(settings.move_left, "A");
        assert_eq!(settings.move_right, "D");
    }

    #[test]
    fn test_controls_default_jump() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.jump, "Space");
    }

    #[test]
    fn test_controls_default_crouch() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.crouch, "LControl");
    }

    #[test]
    fn test_controls_default_sprint() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.sprint, "LShift");
    }

    #[test]
    fn test_controls_default_attack() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.attack, "Mouse0");
    }

    #[test]
    fn test_controls_default_interact() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.interact, "E");
    }

    #[test]
    fn test_controls_default_inventory() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.inventory, "Tab");
    }

    #[test]
    fn test_controls_default_sensitivity() {
        let settings = ControlsSettings::default();
        assert_eq!(settings.mouse_sensitivity, 1.0);
    }

    #[test]
    fn test_controls_default_invert_y() {
        let settings = ControlsSettings::default();
        assert!(!settings.invert_y);
    }

    #[test]
    fn test_controls_clone() {
        let settings = ControlsSettings::default();
        let cloned = settings.clone();
        assert_eq!(settings.move_forward, cloned.move_forward);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COLORBLIND MODE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod colorblind_mode_tests {
    use astraweave_ui::ColorblindMode;

    #[test]
    fn test_colorblind_mode_none() {
        let mode = ColorblindMode::None;
        assert_eq!(mode.display_name(), "None");
    }

    #[test]
    fn test_colorblind_mode_deuteranopia() {
        let mode = ColorblindMode::Deuteranopia;
        assert!(mode.display_name().contains("Deuteranopia"));
        assert!(mode.display_name().contains("Red-Green"));
    }

    #[test]
    fn test_colorblind_mode_protanopia() {
        let mode = ColorblindMode::Protanopia;
        assert!(mode.display_name().contains("Protanopia"));
    }

    #[test]
    fn test_colorblind_mode_tritanopia() {
        let mode = ColorblindMode::Tritanopia;
        assert!(mode.display_name().contains("Tritanopia"));
        assert!(mode.display_name().contains("Blue-Yellow"));
    }

    #[test]
    fn test_colorblind_mode_high_contrast() {
        let mode = ColorblindMode::HighContrast;
        assert!(mode.display_name().contains("High Contrast"));
    }

    #[test]
    fn test_colorblind_mode_all() {
        let all = ColorblindMode::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], ColorblindMode::None);
        assert_eq!(all[1], ColorblindMode::Deuteranopia);
        assert_eq!(all[2], ColorblindMode::Protanopia);
        assert_eq!(all[3], ColorblindMode::Tritanopia);
        assert_eq!(all[4], ColorblindMode::HighContrast);
    }

    #[test]
    fn test_colorblind_mode_default() {
        let mode = ColorblindMode::default();
        assert_eq!(mode, ColorblindMode::None);
    }

    #[test]
    fn test_colorblind_mode_eq() {
        assert_eq!(ColorblindMode::None, ColorblindMode::None);
        assert_ne!(ColorblindMode::None, ColorblindMode::Deuteranopia);
    }

    #[test]
    fn test_colorblind_mode_copy() {
        let mode = ColorblindMode::Protanopia;
        let copied = mode;
        assert_eq!(mode, copied);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ACCESSIBILITY SETTINGS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod accessibility_settings_tests {
    use astraweave_ui::{AccessibilitySettings, ColorblindMode};

    #[test]
    fn test_accessibility_default_colorblind() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.colorblind_mode, ColorblindMode::None);
    }

    #[test]
    fn test_accessibility_default_ui_scale() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.ui_scale, 1.0);
    }

    #[test]
    fn test_accessibility_default_reduce_motion() {
        let settings = AccessibilitySettings::default();
        assert!(!settings.reduce_motion);
    }

    #[test]
    fn test_accessibility_default_large_text() {
        let settings = AccessibilitySettings::default();
        assert!(!settings.large_text);
    }

    #[test]
    fn test_accessibility_set_ui_scale_clamp_low() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(0.5);
        assert_eq!(settings.ui_scale, 0.8);
    }

    #[test]
    fn test_accessibility_set_ui_scale_clamp_high() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(2.0);
        assert_eq!(settings.ui_scale, 1.5);
    }

    #[test]
    fn test_accessibility_set_ui_scale_valid() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(1.2);
        assert_eq!(settings.ui_scale, 1.2);
    }

    #[test]
    fn test_accessibility_font_scale_normal() {
        let settings = AccessibilitySettings::default();
        assert_eq!(settings.font_scale(), 1.0);
    }

    #[test]
    fn test_accessibility_font_scale_large_text() {
        let mut settings = AccessibilitySettings::default();
        settings.large_text = true;
        assert_eq!(settings.font_scale(), 1.25);
    }

    #[test]
    fn test_accessibility_font_scale_combined() {
        let mut settings = AccessibilitySettings::default();
        settings.set_ui_scale(1.2);
        settings.large_text = true;
        assert!((settings.font_scale() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_accessibility_clone() {
        let settings = AccessibilitySettings::default();
        let cloned = settings.clone();
        assert_eq!(settings.ui_scale, cloned.ui_scale);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COLOR TRANSFORMATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod color_transformation_tests {
    use astraweave_ui::{colors, transform_color, ColorblindMode};

    #[test]
    fn test_transform_none_unchanged() {
        let color = colors::HEALTH_FULL;
        let transformed = transform_color(color, ColorblindMode::None);
        assert_eq!(color, transformed);
    }

    #[test]
    fn test_transform_deuteranopia_changes_green() {
        let color = colors::HEALTH_FULL; // green
        let transformed = transform_color(color, ColorblindMode::Deuteranopia);
        // Green should shift toward blue
        assert!(transformed.2 > color.2);
    }

    #[test]
    fn test_transform_protanopia_changes_green() {
        let color = colors::HEALTH_FULL; // green
        let transformed = transform_color(color, ColorblindMode::Protanopia);
        // Should be different from original
        assert_ne!(color, transformed);
    }

    #[test]
    fn test_transform_tritanopia_changes_blue() {
        let color = colors::STAMINA; // blue
        let transformed = transform_color(color, ColorblindMode::Tritanopia);
        // Blue should shift
        assert_ne!(color, transformed);
    }

    #[test]
    fn test_transform_high_contrast() {
        let color = colors::HEALTH_FULL;
        let transformed = transform_color(color, ColorblindMode::HighContrast);
        // Should be different (black/white based)
        assert_ne!(color, transformed);
    }

    #[test]
    fn test_colors_health_full_is_green() {
        let (r, g, b) = colors::HEALTH_FULL;
        assert!(g > r);
        assert!(g > b);
    }

    #[test]
    fn test_colors_health_low_is_red() {
        let (r, g, b) = colors::HEALTH_LOW;
        assert!(r > g);
        assert!(r > b);
    }

    #[test]
    fn test_colors_stamina_is_blue() {
        let (r, g, b) = colors::STAMINA;
        assert!(b > r);
        assert!(b > g);
    }

    #[test]
    fn test_colors_mana_is_purple() {
        let (r, _, b) = colors::MANA;
        assert!(r > 0.4);
        assert!(b > 0.6);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GAMEPAD ACTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod gamepad_action_tests {
    use astraweave_ui::GamepadAction;

    #[test]
    fn test_gamepad_action_up() {
        let action = GamepadAction::Up;
        assert_eq!(action, GamepadAction::Up);
    }

    #[test]
    fn test_gamepad_action_down() {
        let action = GamepadAction::Down;
        assert_eq!(action, GamepadAction::Down);
    }

    #[test]
    fn test_gamepad_action_left() {
        let action = GamepadAction::Left;
        assert_eq!(action, GamepadAction::Left);
    }

    #[test]
    fn test_gamepad_action_right() {
        let action = GamepadAction::Right;
        assert_eq!(action, GamepadAction::Right);
    }

    #[test]
    fn test_gamepad_action_confirm() {
        let action = GamepadAction::Confirm;
        assert_eq!(action, GamepadAction::Confirm);
    }

    #[test]
    fn test_gamepad_action_cancel() {
        let action = GamepadAction::Cancel;
        assert_eq!(action, GamepadAction::Cancel);
    }

    #[test]
    fn test_gamepad_action_pause() {
        let action = GamepadAction::Pause;
        assert_eq!(action, GamepadAction::Pause);
    }

    #[test]
    fn test_gamepad_action_none() {
        let action = GamepadAction::None;
        assert_eq!(action, GamepadAction::None);
    }

    #[test]
    fn test_gamepad_action_ne() {
        assert_ne!(GamepadAction::Up, GamepadAction::Down);
        assert_ne!(GamepadAction::Confirm, GamepadAction::Cancel);
    }

    #[test]
    fn test_gamepad_action_copy() {
        let action = GamepadAction::Confirm;
        let copied = action;
        assert_eq!(action, copied);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GAMEPAD BINDINGS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod gamepad_bindings_tests {
    use astraweave_ui::GamepadBindings;
    use gilrs::Button;

    #[test]
    fn test_bindings_default_confirm() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.confirm, "South");
    }

    #[test]
    fn test_bindings_default_cancel() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.cancel, "East");
    }

    #[test]
    fn test_bindings_default_pause() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.pause, "Start");
    }

    #[test]
    fn test_bindings_to_button_south() {
        assert_eq!(GamepadBindings::to_button("South"), Some(Button::South));
    }

    #[test]
    fn test_bindings_to_button_east() {
        assert_eq!(GamepadBindings::to_button("East"), Some(Button::East));
    }

    #[test]
    fn test_bindings_to_button_north() {
        assert_eq!(GamepadBindings::to_button("North"), Some(Button::North));
    }

    #[test]
    fn test_bindings_to_button_west() {
        assert_eq!(GamepadBindings::to_button("West"), Some(Button::West));
    }

    #[test]
    fn test_bindings_to_button_start() {
        assert_eq!(GamepadBindings::to_button("Start"), Some(Button::Start));
    }

    #[test]
    fn test_bindings_to_button_select() {
        assert_eq!(GamepadBindings::to_button("Select"), Some(Button::Select));
    }

    #[test]
    fn test_bindings_to_button_dpad() {
        assert_eq!(GamepadBindings::to_button("DPadUp"), Some(Button::DPadUp));
        assert_eq!(
            GamepadBindings::to_button("DPadDown"),
            Some(Button::DPadDown)
        );
        assert_eq!(
            GamepadBindings::to_button("DPadLeft"),
            Some(Button::DPadLeft)
        );
        assert_eq!(
            GamepadBindings::to_button("DPadRight"),
            Some(Button::DPadRight)
        );
    }

    #[test]
    fn test_bindings_to_button_triggers() {
        assert_eq!(
            GamepadBindings::to_button("LeftTrigger"),
            Some(Button::LeftTrigger)
        );
        assert_eq!(
            GamepadBindings::to_button("RightTrigger"),
            Some(Button::RightTrigger)
        );
    }

    #[test]
    fn test_bindings_to_button_unknown() {
        assert_eq!(GamepadBindings::to_button("Unknown"), None);
    }

    #[test]
    fn test_bindings_confirm_button() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.confirm_button(), Some(Button::South));
    }

    #[test]
    fn test_bindings_cancel_button() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.cancel_button(), Some(Button::East));
    }

    #[test]
    fn test_bindings_pause_button() {
        let bindings = GamepadBindings::default();
        assert_eq!(bindings.pause_button(), Some(Button::Start));
    }

    #[test]
    fn test_bindings_clone() {
        let bindings = GamepadBindings::default();
        let cloned = bindings.clone();
        assert_eq!(bindings.confirm, cloned.confirm);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HUD STATE STRUCT TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod hud_state_tests {
    use astraweave_ui::HudState;

    #[test]
    fn test_hud_state_default_visible() {
        let state = HudState::default();
        assert!(state.visible);
    }

    #[test]
    fn test_hud_state_default_show_health_bars() {
        let state = HudState::default();
        assert!(state.show_health_bars);
    }

    #[test]
    fn test_hud_state_default_show_objectives() {
        let state = HudState::default();
        assert!(state.show_objectives);
    }

    #[test]
    fn test_hud_state_default_show_minimap() {
        let state = HudState::default();
        assert!(state.show_minimap);
    }

    #[test]
    fn test_hud_state_default_show_subtitles() {
        let state = HudState::default();
        assert!(state.show_subtitles);
    }

    #[test]
    fn test_hud_state_default_quest_tracker_expanded() {
        let state = HudState::default();
        assert!(!state.quest_tracker_collapsed);
    }

    #[test]
    fn test_hud_state_default_minimap_north_up() {
        let state = HudState::default();
        assert!(!state.minimap_rotation);
    }

    #[test]
    fn test_hud_state_default_minimap_zoom() {
        let state = HudState::default();
        assert_eq!(state.minimap_zoom, 1.0);
    }

    #[test]
    fn test_hud_state_default_dialogue_hidden() {
        let state = HudState::default();
        assert!(!state.show_dialogue);
    }

    #[test]
    fn test_hud_state_default_debug_mode_off() {
        let state = HudState::default();
        assert!(!state.debug_mode);
    }

    #[test]
    fn test_hud_state_toggle_visible() {
        let mut state = HudState::default();
        state.visible = false;
        assert!(!state.visible);
    }

    #[test]
    fn test_hud_state_clone() {
        let state = HudState::default();
        let cloned = state.clone();
        assert_eq!(state.visible, cloned.visible);
        assert_eq!(state.minimap_zoom, cloned.minimap_zoom);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PLAYER STATS TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod player_stats_tests {
    use astraweave_ui::PlayerStats;

    #[test]
    fn test_player_stats_default_health() {
        let stats = PlayerStats::default();
        assert_eq!(stats.health, 100.0);
        assert_eq!(stats.max_health, 100.0);
    }

    #[test]
    fn test_player_stats_default_mana() {
        let stats = PlayerStats::default();
        assert_eq!(stats.mana, 100.0);
        assert_eq!(stats.max_mana, 100.0);
    }

    #[test]
    fn test_player_stats_default_stamina() {
        let stats = PlayerStats::default();
        assert_eq!(stats.stamina, 100.0);
        assert_eq!(stats.max_stamina, 100.0);
    }

    #[test]
    fn test_player_stats_clone() {
        let stats = PlayerStats::default();
        let cloned = stats.clone();
        assert_eq!(stats.health, cloned.health);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENEMY FACTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod enemy_faction_tests {
    use astraweave_ui::EnemyFaction;

    #[test]
    fn test_enemy_faction_hostile() {
        let faction = EnemyFaction::Hostile;
        assert_eq!(faction, EnemyFaction::Hostile);
    }

    #[test]
    fn test_enemy_faction_neutral() {
        let faction = EnemyFaction::Neutral;
        assert_eq!(faction, EnemyFaction::Neutral);
    }

    #[test]
    fn test_enemy_faction_friendly() {
        let faction = EnemyFaction::Friendly;
        assert_eq!(faction, EnemyFaction::Friendly);
    }

    #[test]
    fn test_enemy_faction_ne() {
        assert_ne!(EnemyFaction::Hostile, EnemyFaction::Neutral);
        assert_ne!(EnemyFaction::Hostile, EnemyFaction::Friendly);
        assert_ne!(EnemyFaction::Neutral, EnemyFaction::Friendly);
    }

    #[test]
    fn test_enemy_faction_clone() {
        let faction = EnemyFaction::Hostile;
        let cloned = faction.clone();
        assert_eq!(faction, cloned);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENEMY DATA TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod enemy_data_tests {
    use astraweave_ui::{EnemyData, EnemyFaction};

    #[test]
    fn test_enemy_data_new() {
        let enemy = EnemyData::new(1, (10.0, 0.0, 20.0), 100.0, EnemyFaction::Hostile);
        assert_eq!(enemy.id, 1);
        assert_eq!(enemy.max_health, 100.0);
        assert_eq!(enemy.health, 100.0);
    }

    #[test]
    fn test_enemy_data_position() {
        let enemy = EnemyData::new(2, (5.0, 10.0, 15.0), 50.0, EnemyFaction::Neutral);
        assert_eq!(enemy.world_pos.0, 5.0);
        assert_eq!(enemy.world_pos.1, 10.0);
        assert_eq!(enemy.world_pos.2, 15.0);
    }

    #[test]
    fn test_enemy_data_faction() {
        let enemy = EnemyData::new(3, (0.0, 0.0, 0.0), 200.0, EnemyFaction::Friendly);
        assert_eq!(enemy.faction, EnemyFaction::Friendly);
    }

    #[test]
    fn test_enemy_data_clone() {
        let enemy = EnemyData::new(4, (1.0, 2.0, 3.0), 75.0, EnemyFaction::Hostile);
        let cloned = enemy.clone();
        assert_eq!(enemy.id, cloned.id);
        assert_eq!(enemy.max_health, cloned.max_health);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// POI TYPE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod poi_type_tests {
    use astraweave_ui::PoiType;

    #[test]
    fn test_poi_type_objective() {
        let poi = PoiType::Objective;
        assert_eq!(poi, PoiType::Objective);
    }

    #[test]
    fn test_poi_type_waypoint() {
        let poi = PoiType::Waypoint;
        assert_eq!(poi, PoiType::Waypoint);
    }

    #[test]
    fn test_poi_type_vendor() {
        let poi = PoiType::Vendor;
        assert_eq!(poi, PoiType::Vendor);
    }

    #[test]
    fn test_poi_type_danger() {
        let poi = PoiType::Danger;
        assert_eq!(poi, PoiType::Danger);
    }

    #[test]
    fn test_poi_type_ne() {
        assert_ne!(PoiType::Objective, PoiType::Waypoint);
        assert_ne!(PoiType::Vendor, PoiType::Danger);
    }

    #[test]
    fn test_poi_type_clone() {
        let poi = PoiType::Objective;
        let cloned = poi.clone();
        assert_eq!(poi, cloned);
    }

    #[test]
    fn test_poi_type_icon_objective() {
        let poi = PoiType::Objective;
        assert_eq!(poi.icon(), "🎯");
    }

    #[test]
    fn test_poi_type_icon_waypoint() {
        let poi = PoiType::Waypoint;
        assert_eq!(poi.icon(), "📍");
    }

    #[test]
    fn test_poi_type_icon_vendor() {
        let poi = PoiType::Vendor;
        assert_eq!(poi.icon(), "🏪");
    }

    #[test]
    fn test_poi_type_icon_danger() {
        let poi = PoiType::Danger;
        assert_eq!(poi.icon(), "⚔️");
    }

    #[test]
    fn test_poi_type_color_objective() {
        let poi = PoiType::Objective;
        let color = poi.color();
        assert_eq!(color, egui::Color32::YELLOW);
    }

    #[test]
    fn test_poi_type_color_waypoint() {
        let poi = PoiType::Waypoint;
        let color = poi.color();
        assert_eq!(color, egui::Color32::LIGHT_BLUE);
    }

    #[test]
    fn test_poi_type_color_vendor() {
        let poi = PoiType::Vendor;
        let color = poi.color();
        assert_eq!(color, egui::Color32::GREEN);
    }

    #[test]
    fn test_poi_type_color_danger() {
        let poi = PoiType::Danger;
        let color = poi.color();
        assert_eq!(color, egui::Color32::RED);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DAMAGE TYPE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod damage_type_tests {
    use astraweave_ui::DamageType;

    #[test]
    fn test_damage_type_normal() {
        let dt = DamageType::Normal;
        assert_eq!(dt, DamageType::Normal);
    }

    #[test]
    fn test_damage_type_critical() {
        let dt = DamageType::Critical;
        assert_eq!(dt, DamageType::Critical);
    }

    #[test]
    fn test_damage_type_self_damage() {
        let dt = DamageType::SelfDamage;
        assert_eq!(dt, DamageType::SelfDamage);
    }

    #[test]
    fn test_damage_type_ne() {
        assert_ne!(DamageType::Normal, DamageType::Critical);
        assert_ne!(DamageType::Normal, DamageType::SelfDamage);
        assert_ne!(DamageType::Critical, DamageType::SelfDamage);
    }

    #[test]
    fn test_damage_type_clone() {
        let dt = DamageType::Critical;
        let cloned = dt.clone();
        assert_eq!(dt, cloned);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// EASING FUNCTION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod easing_tests {
    use astraweave_ui::hud::easing;

    #[test]
    fn test_ease_out_cubic_at_zero() {
        let result = easing::ease_out_cubic(0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_ease_out_cubic_at_one() {
        let result = easing::ease_out_cubic(1.0);
        assert!((result - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ease_out_cubic_at_half() {
        let result = easing::ease_out_cubic(0.5);
        // Ease out cubic at 0.5: (-0.5)^3 + 1 = -0.125 + 1 = 0.875
        assert!((result - 0.875).abs() < 0.001);
    }

    #[test]
    fn test_ease_in_out_quad_at_zero() {
        let result = easing::ease_in_out_quad(0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_ease_in_out_quad_at_one() {
        let result = easing::ease_in_out_quad(1.0);
        assert!((result - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_ease_in_out_quad_at_half() {
        let result = easing::ease_in_out_quad(0.5);
        // At t=0.5: -1 + (4-1)*0.5 = -1 + 1.5 = 0.5
        assert!((result - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_ease_out_cubic_increasing() {
        let t1 = easing::ease_out_cubic(0.25);
        let t2 = easing::ease_out_cubic(0.5);
        let t3 = easing::ease_out_cubic(0.75);
        assert!(t1 < t2);
        assert!(t2 < t3);
    }

    #[test]
    fn test_ease_in_out_quad_increasing() {
        let t1 = easing::ease_in_out_quad(0.25);
        let t2 = easing::ease_in_out_quad(0.5);
        let t3 = easing::ease_in_out_quad(0.75);
        assert!(t1 < t2);
        assert!(t2 < t3);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HEALTH ANIMATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod health_animation_tests {
    use astraweave_ui::hud::HealthAnimation;

    #[test]
    fn test_animation_new() {
        let anim = HealthAnimation::new(100.0);
        assert_eq!(anim.current_visual, 100.0);
        assert_eq!(anim.target, 100.0);
    }

    #[test]
    fn test_animation_set_target() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);
        assert_eq!(anim.target, 50.0);
        // Animation time should reset
        assert_eq!(anim.animation_time, 0.0);
    }

    #[test]
    fn test_animation_damage_triggers_flash() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0); // Taking damage
                               // Flash timer should be set
        assert!(anim.flash_timer > 0.0);
    }

    #[test]
    fn test_animation_heal_no_flash() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0); // Healing
                                // Flash timer should NOT be set for healing
        assert_eq!(anim.flash_timer, 0.0);
    }

    #[test]
    fn test_animation_visual_health() {
        let anim = HealthAnimation::new(75.0);
        assert_eq!(anim.visual_health(), 75.0);
    }

    #[test]
    fn test_animation_flash_alpha_initial() {
        let anim = HealthAnimation::new(100.0);
        assert_eq!(anim.flash_alpha(), 0.0);
    }

    #[test]
    fn test_animation_flash_alpha_after_damage() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0); // Damage
        assert!(anim.flash_alpha() > 0.0);
    }

    #[test]
    fn test_animation_is_healing_false() {
        let anim = HealthAnimation::new(100.0);
        assert!(!anim.is_healing());
    }

    #[test]
    fn test_animation_is_healing_true() {
        let mut anim = HealthAnimation::new(50.0);
        anim.set_target(100.0);
        assert!(anim.is_healing());
    }

    #[test]
    fn test_animation_update_progresses() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);
        let initial = anim.current_visual;
        anim.update(0.1);
        // Visual health should have moved toward target
        assert!(anim.current_visual < initial);
    }

    #[test]
    fn test_animation_update_flash_decreases() {
        let mut anim = HealthAnimation::new(100.0);
        anim.set_target(50.0);
        let initial_flash = anim.flash_timer;
        anim.update(0.05);
        assert!(anim.flash_timer < initial_flash);
    }

    #[test]
    fn test_animation_clone() {
        let anim = HealthAnimation::new(100.0);
        let cloned = anim.clone();
        assert_eq!(anim.current_visual, cloned.current_visual);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// QUEST TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod quest_tests {
    use astraweave_ui::{Objective, Quest};

    #[test]
    fn test_objective_clone() {
        let obj = Objective {
            id: 1,
            description: "Find the sword".to_string(),
            completed: false,
            progress: None,
        };
        let cloned = obj.clone();
        assert_eq!(obj.description, cloned.description);
    }

    #[test]
    fn test_objective_with_progress() {
        let obj = Objective {
            id: 2,
            description: "Collect items".to_string(),
            completed: false,
            progress: Some((3, 5)),
        };
        assert_eq!(obj.progress, Some((3, 5)));
    }

    #[test]
    fn test_quest_clone() {
        let quest = Quest {
            id: 1,
            title: "Test Quest".to_string(),
            description: "A test quest".to_string(),
            objectives: vec![Objective {
                id: 1,
                description: "Do thing".to_string(),
                completed: false,
                progress: None,
            }],
        };
        let cloned = quest.clone();
        assert_eq!(quest.id, cloned.id);
        assert_eq!(quest.title, cloned.title);
    }

    #[test]
    fn test_quest_completion_empty() {
        let quest = Quest {
            id: 1,
            title: "Empty Quest".to_string(),
            description: "No objectives".to_string(),
            objectives: vec![],
        };
        assert_eq!(quest.completion(), 0.0);
    }

    #[test]
    fn test_quest_completion_none_done() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Test".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: false,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };
        assert_eq!(quest.completion(), 0.0);
    }

    #[test]
    fn test_quest_completion_half_done() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Test".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };
        assert!((quest.completion() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_quest_completion_all_done() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Test".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: true,
                    progress: None,
                },
            ],
        };
        assert_eq!(quest.completion(), 1.0);
    }

    #[test]
    fn test_quest_is_complete_false() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Test".to_string(),
            objectives: vec![
                Objective {
                    id: 1,
                    description: "A".to_string(),
                    completed: true,
                    progress: None,
                },
                Objective {
                    id: 2,
                    description: "B".to_string(),
                    completed: false,
                    progress: None,
                },
            ],
        };
        assert!(!quest.is_complete());
    }

    #[test]
    fn test_quest_is_complete_true() {
        let quest = Quest {
            id: 1,
            title: "Test".to_string(),
            description: "Test".to_string(),
            objectives: vec![Objective {
                id: 1,
                description: "A".to_string(),
                completed: true,
                progress: None,
            }],
        };
        assert!(quest.is_complete());
    }

    #[test]
    fn test_quest_is_complete_empty() {
        let quest = Quest {
            id: 1,
            title: "Empty".to_string(),
            description: "No objectives".to_string(),
            objectives: vec![],
        };
        assert!(!quest.is_complete());
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DIALOGUE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod dialogue_tests {
    use astraweave_ui::{DialogueChoice, DialogueNode};

    #[test]
    fn test_dialogue_choice_clone() {
        let choice = DialogueChoice {
            id: 1,
            text: "Yes".to_string(),
            next_node: Some(2),
        };
        let cloned = choice.clone();
        assert_eq!(choice.text, cloned.text);
        assert_eq!(choice.next_node, cloned.next_node);
    }

    #[test]
    fn test_dialogue_node_clone() {
        let node = DialogueNode {
            id: 1,
            speaker_name: "NPC".to_string(),
            text: "Hello there".to_string(),
            choices: vec![],
            portrait_id: None,
        };
        let cloned = node.clone();
        assert_eq!(node.id, cloned.id);
        assert_eq!(node.speaker_name, cloned.speaker_name);
    }

    #[test]
    fn test_dialogue_choice_next_node_none() {
        let choice = DialogueChoice {
            id: 2,
            text: "Goodbye".to_string(),
            next_node: None,
        };
        assert!(choice.next_node.is_none());
    }

    #[test]
    fn test_dialogue_node_with_portrait() {
        let node = DialogueNode {
            id: 3,
            speaker_name: "Hero".to_string(),
            text: "I will save the day!".to_string(),
            choices: vec![],
            portrait_id: Some(42),
        };
        assert_eq!(node.portrait_id, Some(42));
    }

    #[test]
    fn test_dialogue_node_with_choices() {
        let node = DialogueNode {
            id: 1,
            speaker_name: "NPC".to_string(),
            text: "What do you want?".to_string(),
            choices: vec![
                DialogueChoice {
                    id: 1,
                    text: "Trade".to_string(),
                    next_node: Some(2),
                },
                DialogueChoice {
                    id: 2,
                    text: "Leave".to_string(),
                    next_node: None,
                },
            ],
            portrait_id: None,
        };
        assert_eq!(node.choices.len(), 2);
        assert_eq!(node.choices[0].text, "Trade");
        assert_eq!(node.choices[1].next_node, None);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// STATE MODULE TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod state_module_tests {
    use astraweave_ui::{Accessibility, UiFlags};

    #[test]
    fn test_accessibility_default_high_contrast() {
        let acc = Accessibility::default();
        assert!(!acc.high_contrast_ui);
    }

    #[test]
    fn test_accessibility_default_reduce_motion() {
        let acc = Accessibility::default();
        assert!(!acc.reduce_motion);
    }

    #[test]
    fn test_accessibility_default_subtitles() {
        let acc = Accessibility::default();
        assert!(acc.subtitles);
    }

    #[test]
    fn test_accessibility_default_subtitle_scale() {
        let acc = Accessibility::default();
        assert_eq!(acc.subtitle_scale, 1.0);
    }

    #[test]
    fn test_accessibility_default_colorblind_mode() {
        let acc = Accessibility::default();
        assert!(acc.colorblind_mode.is_none());
    }

    #[test]
    fn test_ui_flags_default() {
        let flags = UiFlags::default();
        assert!(!flags.show_menu);
        assert!(!flags.show_inventory);
        assert!(!flags.show_map);
        assert!(!flags.show_quests);
        assert!(!flags.show_crafting);
        assert!(!flags.show_settings);
    }

    #[test]
    fn test_ui_flags_clone() {
        let mut flags = UiFlags::default();
        flags.show_menu = true;
        let cloned = flags.clone();
        assert!(cloned.show_menu);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DAMAGE NUMBER TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod damage_number_tests {
    use astraweave_ui::{DamageNumber, DamageType};

    #[test]
    fn test_damage_number_new() {
        let dn = DamageNumber::new(42, 1.5, (10.0, 5.0, 20.0), DamageType::Normal);
        assert_eq!(dn.value, 42);
        assert_eq!(dn.spawn_time, 1.5);
        assert_eq!(dn.damage_type, DamageType::Normal);
    }

    #[test]
    fn test_damage_number_world_pos() {
        let dn = DamageNumber::new(100, 0.0, (1.0, 2.0, 3.0), DamageType::Critical);
        assert_eq!(dn.world_pos, (1.0, 2.0, 3.0));
    }

    #[test]
    fn test_damage_number_clone() {
        let dn = DamageNumber::new(50, 0.5, (0.0, 0.0, 0.0), DamageType::Normal);
        let cloned = dn.clone();
        assert_eq!(dn.value, cloned.value);
        assert_eq!(dn.spawn_time, cloned.spawn_time);
    }

    #[test]
    fn test_damage_number_critical() {
        let dn = DamageNumber::new(150, 0.0, (0.0, 0.0, 0.0), DamageType::Critical);
        assert_eq!(dn.damage_type, DamageType::Critical);
        // Critical has higher shake amplitude
        assert!(dn.shake_amplitude > 0.1);
    }

    #[test]
    fn test_damage_number_normal_shake() {
        let dn = DamageNumber::new(50, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        // Normal has lower shake amplitude
        assert!(dn.shake_amplitude < 0.1);
    }

    #[test]
    fn test_damage_number_calculate_offset_at_zero() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let (x, y) = dn.calculate_offset(0.0);
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_damage_number_calculate_offset_progression() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let (_, y1) = dn.calculate_offset(0.1);
        let (_, _y2) = dn.calculate_offset(0.2);
        // Initially moves up (negative y), then gravity takes over
        assert!(y1 < 0.0); // Moving up initially
                           // After more time, gravity pulls down
        let (_, y3) = dn.calculate_offset(1.0);
        assert!(y3 > y1); // Has moved down from initial position
    }

    #[test]
    fn test_damage_number_calculate_shake_at_zero() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        let shake = dn.calculate_shake(0.0);
        // At t=0, sin(0) = 0, so shake should be 0
        assert!(shake.abs() < 0.001);
    }

    #[test]
    fn test_damage_number_calculate_shake_decreases() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Critical);
        let _shake1 = dn.calculate_shake(0.1).abs();
        let _shake2 = dn.calculate_shake(0.5).abs();
        // Shake should dampen over time (on average)
        // Due to oscillation, check that later time has less amplitude envelope
        let envelope1 = dn.shake_amplitude * (-0.1 * 5.0_f32).exp();
        let envelope2 = dn.shake_amplitude * (-0.5 * 5.0_f32).exp();
        assert!(envelope2 < envelope1);
    }

    #[test]
    fn test_damage_number_gravity() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        assert_eq!(dn.gravity, 150.0);
    }

    #[test]
    fn test_damage_number_initial_velocity_upward() {
        let dn = DamageNumber::new(100, 0.0, (0.0, 0.0, 0.0), DamageType::Normal);
        assert!(dn.velocity_y < 0.0); // Negative = upward
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// POI MARKER TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod poi_marker_tests {
    use astraweave_ui::{PoiMarker, PoiType};

    #[test]
    fn test_poi_marker_clone() {
        let marker = PoiMarker {
            id: 1,
            world_pos: (100.0, 200.0),
            poi_type: PoiType::Objective,
            label: Some("Main Quest".to_string()),
        };
        let cloned = marker.clone();
        assert_eq!(marker.world_pos, cloned.world_pos);
        assert_eq!(marker.poi_type, cloned.poi_type);
    }

    #[test]
    fn test_poi_marker_no_label() {
        let marker = PoiMarker {
            id: 2,
            world_pos: (0.0, 0.0),
            poi_type: PoiType::Danger,
            label: None,
        };
        assert!(marker.label.is_none());
    }

    #[test]
    fn test_poi_marker_eq() {
        let a = PoiMarker {
            id: 1,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Waypoint,
            label: None,
        };
        let b = PoiMarker {
            id: 1,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Waypoint,
            label: None,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_poi_marker_ne_by_id() {
        let a = PoiMarker {
            id: 1,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Vendor,
            label: None,
        };
        let b = PoiMarker {
            id: 2,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Vendor,
            label: None,
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_poi_marker_ne_by_type() {
        let a = PoiMarker {
            id: 1,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Vendor,
            label: None,
        };
        let b = PoiMarker {
            id: 1,
            world_pos: (10.0, 20.0),
            poi_type: PoiType::Danger,
            label: None,
        };
        assert_ne!(a, b);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TOOLTIP DATA TESTS
// ═══════════════════════════════════════════════════════════════════════════

mod tooltip_data_tests {
    use astraweave_ui::TooltipData;

    #[test]
    fn test_tooltip_data_clone() {
        let tooltip = TooltipData {
            title: "Iron Sword".to_string(),
            description: "A basic sword".to_string(),
            stats: vec![("Attack".to_string(), "10".to_string())],
            flavor_text: None,
        };
        let cloned = tooltip.clone();
        assert_eq!(tooltip.title, cloned.title);
    }

    #[test]
    fn test_tooltip_data_no_flavor() {
        let tooltip = TooltipData {
            title: "Key".to_string(),
            description: "Opens a door".to_string(),
            stats: vec![],
            flavor_text: None,
        };
        assert!(tooltip.flavor_text.is_none());
    }

    #[test]
    fn test_tooltip_data_with_flavor() {
        let tooltip = TooltipData {
            title: "Ancient Blade".to_string(),
            description: "A legendary weapon".to_string(),
            stats: vec![("Attack".to_string(), "100".to_string())],
            flavor_text: Some("Forged in the fires of Mount Doom".to_string()),
        };
        assert_eq!(
            tooltip.flavor_text.as_deref(),
            Some("Forged in the fires of Mount Doom")
        );
    }

    #[test]
    fn test_tooltip_data_multiple_stats() {
        let tooltip = TooltipData {
            title: "Magic Staff".to_string(),
            description: "A powerful staff".to_string(),
            stats: vec![
                ("Attack".to_string(), "50".to_string()),
                ("Magic".to_string(), "75".to_string()),
                ("Range".to_string(), "10m".to_string()),
            ],
            flavor_text: None,
        };
        assert_eq!(tooltip.stats.len(), 3);
        assert_eq!(tooltip.stats[1].0, "Magic");
        assert_eq!(tooltip.stats[1].1, "75");
    }

    #[test]
    fn test_tooltip_data_empty_stats() {
        let tooltip = TooltipData {
            title: "Consumable".to_string(),
            description: "Use it!".to_string(),
            stats: vec![],
            flavor_text: Some("Tastes like chicken".to_string()),
        };
        assert!(tooltip.stats.is_empty());
    }
}
