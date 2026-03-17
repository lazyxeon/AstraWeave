#![cfg(feature = "kittest")]

use aw_editor_lib::panel_type::PanelType;
use aw_editor_lib::ui::menu_bar::{
    AlignDirection, DistributeDirection, MenuActionHandler, MenuBar,
};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct MockHandler {
    new_called: bool,
    undo_called: bool,
    selection_count: usize,
    group_called: bool,
}

impl MenuActionHandler for MockHandler {
    fn on_new(&mut self) {
        self.new_called = true;
    }
    fn on_open(&mut self) {}
    fn on_save(&mut self) {}
    fn on_save_json(&mut self) {}
    fn on_save_scene(&mut self) {}
    fn on_load_scene(&mut self) {}

    fn on_undo(&mut self) {
        self.undo_called = true;
    }
    fn on_redo(&mut self) {}
    fn on_delete(&mut self) {}

    fn selection_count(&self) -> usize {
        self.selection_count
    }
    fn on_apply_material(&mut self) {}
    fn on_group_selection(&mut self) {
        self.group_called = true;
    }
    fn on_ungroup_selection(&mut self) {}
    fn on_align_selection(&mut self, _dir: AlignDirection) {}
    fn on_distribute_selection(&mut self, _dir: DistributeDirection) {}
    fn on_select_all(&mut self) {}
    fn on_deselect_all(&mut self) {}

    // Recent Files
    fn get_recent_files(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    fn on_open_recent(&mut self, _path: PathBuf) {}
    fn on_clear_recent(&mut self) {}

    // View
    fn is_view_hierarchy_open(&self) -> bool {
        true
    }
    fn toggle_view_hierarchy(&mut self) {}
    fn is_view_inspector_open(&self) -> bool {
        true
    }
    fn toggle_view_inspector(&mut self) {}
    fn is_view_console_open(&self) -> bool {
        true
    }
    fn toggle_view_console(&mut self) {}
    fn is_grid_visible(&self) -> bool {
        true
    }
    fn toggle_grid(&mut self) {}

    // Window
    fn is_docking_enabled(&self) -> bool {
        true
    }
    fn toggle_docking(&mut self) {}
    fn on_apply_layout_preset(&mut self, _preset_name: &str) {}
    fn is_dock_panel_visible(&self, _panel: PanelType) -> bool {
        true
    }
    fn toggle_dock_panel(&mut self, _panel: PanelType) {}

    // Settings
    fn on_open_settings(&mut self) {}

    // Debug
    fn on_scan_for_models(&mut self) {}
    fn on_load_test_model(&mut self, _name: &str, _path: PathBuf) {}
    fn on_toggle_engine_rendering(&mut self) {}
    fn on_show_engine_info(&mut self) {}

    fn on_debug_material(&mut self, _name: &str) {}
    fn on_debug_time_set(&mut self, _time: f32) {}
    fn get_time_of_day(&self) -> f32 {
        12.0
    }
    fn get_time_period(&self) -> String {
        "Day".to_string()
    }

    fn is_shadows_enabled(&self) -> bool {
        true
    }
    fn set_shadows_enabled(&mut self, _enabled: bool) {}

    fn on_diff_assets(&mut self) {}
    fn on_clear_console(&mut self) {}
}

#[test]
fn test_menu_new_button() {
    let handler = Arc::new(Mutex::new(MockHandler {
        new_called: false,
        undo_called: false,
        selection_count: 0,
        group_called: false,
    }));
    let handler_clone = handler.clone();

    let mut harness = Harness::new_ui(move |ui| {
        let mut h = handler_clone.lock().unwrap();
        MenuBar::show(ui, &mut *h);
    });

    harness.get_by_label("New").click();
    harness.run();

    assert!(
        handler.lock().unwrap().new_called,
        "New button should trigger handler"
    );
}

#[test]
fn test_menu_edit_undo() {
    let handler = Arc::new(Mutex::new(MockHandler {
        new_called: false,
        undo_called: false,
        selection_count: 0,
        group_called: false,
    }));
    let handler_clone = handler.clone();

    let mut harness = Harness::new_ui(move |ui| {
        let mut h = handler_clone.lock().unwrap();
        MenuBar::show(ui, &mut *h);
    });

    // Click "Edit" menu button to open popup
    harness.get_by_label("✏️ Edit").click();
    harness.run();

    // Now look for Undo in the popup
    harness.get_by_label("↩️ Undo (Ctrl+Z)").click();
    harness.run();

    assert!(
        handler.lock().unwrap().undo_called,
        "Undo button in menu should trigger handler"
    );
}

#[test]
fn test_menu_selection_state() {
    // Test with selection > 1 (should enable group button)
    let handler = Arc::new(Mutex::new(MockHandler {
        new_called: false,
        undo_called: false,
        selection_count: 2,
        group_called: false,
    }));
    let handler_clone = handler.clone();

    let mut harness = Harness::new_ui(move |ui| {
        let mut h = handler_clone.lock().unwrap();
        MenuBar::show(ui, &mut *h);
    });

    harness.get_by_label("✏️ Edit").click();
    harness.run();

    // Check if Group Selection is clickable
    harness.get_by_label("📁 Group Selection (Ctrl+G)").click();
    harness.run();

    assert!(
        handler.lock().unwrap().group_called,
        "Group selection should be called when items selected"
    );
}
