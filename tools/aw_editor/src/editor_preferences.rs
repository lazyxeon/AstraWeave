use crate::gizmo::snapping::SnappingConfig;
use crate::viewport::camera::OrbitCamera;
use serde::{Deserialize, Serialize};
use std::fs;

const PREFERENCES_PATH: &str = ".editor_preferences.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPreferences {
    pub show_grid: bool,
    pub auto_save_enabled: bool,
    pub auto_save_interval_secs: f32,
    // Week 7: Enhanced auto-save settings
    #[serde(default = "default_auto_save_count")]
    pub auto_save_keep_count: usize,
    #[serde(default)]
    pub auto_save_to_separate_dir: bool,
    pub show_hierarchy_panel: bool,
    pub show_inspector_panel: bool,
    pub show_console_panel: bool,
    pub camera: Option<OrbitCamera>,
    pub snapping: Option<SnappingConfig>,
    /// Serialized dock layout JSON for persistence
    #[serde(default)]
    pub layout_json: Option<String>,
    /// Whether the first-run tutorial has been completed or skipped.
    #[serde(default)]
    pub tutorial_completed: bool,
    /// Directories to scan for .blend scene assets.
    #[serde(default = "default_blend_asset_directories")]
    pub blend_asset_directories: Vec<String>,
}

fn default_auto_save_count() -> usize {
    3
}

pub fn default_blend_asset_directories() -> Vec<String> {
    vec![
        "assets/blends".to_string(),
        "assets/scenes".to_string(),
    ]
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            show_grid: true,
            auto_save_enabled: false,
            auto_save_interval_secs: 300.0,
            auto_save_keep_count: 3,
            auto_save_to_separate_dir: true,
            show_hierarchy_panel: true,
            show_inspector_panel: true,
            show_console_panel: true,
            camera: None,
            snapping: None,
            layout_json: None,
            tutorial_completed: false,
            blend_asset_directories: default_blend_asset_directories(),
        }
    }
}

impl EditorPreferences {
    pub fn load() -> Self {
        if let Ok(contents) = fs::read_to_string(PREFERENCES_PATH) {
            if let Ok(prefs) = serde_json::from_str(&contents) {
                return prefs;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        match serde_json::to_string_pretty(&self) {
            Ok(json) => {
                if let Err(e) = fs::write(PREFERENCES_PATH, json) {
                    tracing::error!(
                        "Failed to write editor preferences to {}: {}",
                        PREFERENCES_PATH,
                        e
                    );
                }
            }
            Err(e) => {
                tracing::error!("Failed to serialize editor preferences: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = EditorPreferences::default();
        assert!(prefs.show_grid);
        assert!(!prefs.auto_save_enabled);
        assert_eq!(prefs.auto_save_interval_secs, 300.0);
        assert!(prefs.show_hierarchy_panel);
        assert!(prefs.show_inspector_panel);
        assert!(prefs.show_console_panel);
    }

    #[test]
    fn test_week7_auto_save_defaults() {
        let prefs = EditorPreferences::default();
        assert_eq!(prefs.auto_save_keep_count, 3);
        assert!(prefs.auto_save_to_separate_dir);
    }

    #[test]
    fn test_preferences_serialization_roundtrip() {
        let prefs = EditorPreferences {
            show_grid: false,
            auto_save_enabled: true,
            auto_save_interval_secs: 120.0,
            auto_save_keep_count: 5,
            auto_save_to_separate_dir: false,
            show_hierarchy_panel: false,
            show_inspector_panel: true,
            show_console_panel: false,
            camera: None,
            snapping: None,
            layout_json: None,
            tutorial_completed: false,
            blend_asset_directories: vec!["test/blends".into()],
        };

        let json = serde_json::to_string(&prefs).expect("serialize");
        let restored: EditorPreferences = serde_json::from_str(&json).expect("deserialize");

        assert!(!restored.show_grid);
        assert!(restored.auto_save_enabled);
        assert_eq!(restored.auto_save_interval_secs, 120.0);
        assert_eq!(restored.auto_save_keep_count, 5);
        assert!(!restored.auto_save_to_separate_dir);
    }

    #[test]
    fn test_preferences_deserialize_missing_week7_fields() {
        // Simulates loading old preferences without Week 7 fields
        let old_json = r#"{
            "show_grid": true,
            "auto_save_enabled": false,
            "auto_save_interval_secs": 300.0,
            "show_hierarchy_panel": true,
            "show_inspector_panel": true,
            "show_console_panel": true,
            "camera": null,
            "snapping": null
        }"#;

        let prefs: EditorPreferences = serde_json::from_str(old_json).expect("deserialize");

        // Should use defaults for missing Week 7 fields
        assert_eq!(prefs.auto_save_keep_count, 3);
        assert!(!prefs.auto_save_to_separate_dir); // default is false when missing
    }

    #[test]
    fn test_default_auto_save_count_fn() {
        assert_eq!(default_auto_save_count(), 3);
    }
}
