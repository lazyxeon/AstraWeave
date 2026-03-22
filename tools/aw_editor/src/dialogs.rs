//! Dialog rendering for the AstraWeave Editor
//!
//! This module contains helper functions for rendering modal dialogs
//! such as quit confirmation, new scene confirmation, settings, and help.

use eframe::egui;

/// Helper to render a modal overlay (semi-transparent background)
pub fn show_modal_overlay(ctx: &egui::Context, id: &str) {
    let screen_rect = ctx.screen_rect();
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Background)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128),
            );
        });
}

/// Keyboard shortcuts data for help and settings dialogs
pub const KEYBOARD_SHORTCUTS: &[(&str, &str, &str)] = &[
    // Category, Shortcut, Description
    ("File", "Ctrl+N", "New Scene"),
    ("File", "Ctrl+S", "Save Scene"),
    ("File", "Ctrl+Shift+S", "Save As"),
    ("File", "Ctrl+O", "Open Scene"),
    ("Edit", "Ctrl+Z", "Undo"),
    ("Edit", "Ctrl+Y", "Redo"),
    ("Edit", "Ctrl+Shift+Z", "Redo (Alt)"),
    ("Edit", "Ctrl+A", "Select All"),
    ("Edit", "Ctrl+D", "Duplicate"),
    ("Edit", "Ctrl+C", "Copy"),
    ("Edit", "Ctrl+V", "Paste"),
    ("Edit", "Delete", "Delete Selected"),
    ("Edit", "Escape", "Deselect All"),
    ("Camera", "F", "Focus on Selected"),
    ("Camera", "Home", "Reset Camera"),
    ("Camera", "Alt+1", "Front View"),
    ("Camera", "Alt+3", "Right View"),
    ("Camera", "Alt+7", "Top View"),
    ("Camera", "Alt+0", "Perspective View"),
    ("Gizmo", "G", "Translate Mode"),
    ("Gizmo", "R", "Rotate Mode"),
    ("Gizmo", "S", "Scale Mode"),
    ("Play", "F5", "Play"),
    ("Play", "F6", "Pause"),
    ("Play", "F7", "Stop"),
    ("Play", "F8", "Step Frame"),
    ("View", "F1", "Show Help"),
    ("View", "G", "Toggle Grid"),
];

/// Get the total number of keyboard shortcuts
pub fn shortcut_count() -> usize {
    KEYBOARD_SHORTCUTS.len()
}

/// Get all unique categories
pub fn shortcut_categories() -> Vec<&'static str> {
    let mut categories: Vec<&'static str> =
        KEYBOARD_SHORTCUTS.iter().map(|(cat, _, _)| *cat).collect();
    categories.dedup();
    categories
}

/// Get shortcuts for a specific category
pub fn shortcuts_for_category(category: &str) -> Vec<(&'static str, &'static str)> {
    KEYBOARD_SHORTCUTS
        .iter()
        .filter(|(cat, _, _)| *cat == category)
        .map(|(_, shortcut, desc)| (*shortcut, *desc))
        .collect()
}

/// Find a shortcut by its key combination
pub fn find_shortcut(shortcut: &str) -> Option<(&'static str, &'static str)> {
    KEYBOARD_SHORTCUTS
        .iter()
        .find(|(_, s, _)| *s == shortcut)
        .map(|(cat, _, desc)| (*cat, *desc))
}

/// Search shortcuts by description (case-insensitive)
pub fn search_shortcuts(query: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let query_lower = query.to_lowercase();
    KEYBOARD_SHORTCUTS
        .iter()
        .filter(|(_, _, desc)| desc.to_lowercase().contains(&query_lower))
        .copied()
        .collect()
}

/// Render keyboard shortcuts as a grid
pub fn show_shortcuts_grid(ui: &mut egui::Ui) {
    let mut current_category = "";

    for (category, shortcut, description) in KEYBOARD_SHORTCUTS {
        if *category != current_category {
            if !current_category.is_empty() {
                ui.add_space(8.0);
            }
            ui.heading(*category);
            current_category = category;
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(*shortcut).monospace().strong());
            ui.add_space(20.0);
            ui.label(*description);
        });
    }
}

/// Render shortcuts in a compact grid format for settings dialog
pub fn show_shortcuts_compact_grid(ui: &mut egui::Ui) {
    egui::Grid::new("shortcuts_grid_compact")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for (_, shortcut, description) in KEYBOARD_SHORTCUTS.iter().take(16) {
                ui.label(*shortcut);
                ui.label(*description);
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcut_count() {
        assert_eq!(shortcut_count(), 28);
    }

    #[test]
    fn test_shortcut_categories() {
        let categories = shortcut_categories();
        assert!(!categories.is_empty());
        assert!(categories.contains(&"File"));
        assert!(categories.contains(&"Edit"));
    }

    #[test]
    fn test_shortcuts_for_category_file() {
        let shortcuts = shortcuts_for_category("File");
        assert!(!shortcuts.is_empty());
        assert!(shortcuts.iter().any(|(s, _)| *s == "Ctrl+N"));
    }

    #[test]
    fn test_shortcuts_for_category_edit() {
        let shortcuts = shortcuts_for_category("Edit");
        assert!(!shortcuts.is_empty());
        assert!(shortcuts.iter().any(|(s, _)| *s == "Ctrl+Z"));
    }

    #[test]
    fn test_shortcuts_for_category_empty() {
        let shortcuts = shortcuts_for_category("NonExistent");
        assert!(shortcuts.is_empty());
    }

    #[test]
    fn test_find_shortcut_existing() {
        let result = find_shortcut("Ctrl+S");
        assert!(result.is_some());
        let (category, desc) = result.unwrap();
        assert_eq!(category, "File");
        assert_eq!(desc, "Save Scene");
    }

    #[test]
    fn test_find_shortcut_not_found() {
        let result = find_shortcut("Ctrl+K");
        assert!(result.is_none());
    }

    #[test]
    fn test_search_shortcuts() {
        let results = search_shortcuts("save");
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|(_, _, desc)| desc.to_lowercase().contains("save")));
    }

    #[test]
    fn test_search_shortcuts_case_insensitive() {
        let results = search_shortcuts("UNDO");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_shortcuts_no_results() {
        let results = search_shortcuts("xyznonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_keyboard_shortcuts_constant() {
        assert!(!KEYBOARD_SHORTCUTS.is_empty());
        for (cat, shortcut, desc) in KEYBOARD_SHORTCUTS {
            assert!(!cat.is_empty());
            assert!(!shortcut.is_empty());
            assert!(!desc.is_empty());
        }
    }
}
