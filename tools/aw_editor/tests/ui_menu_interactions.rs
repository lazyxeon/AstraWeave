//! Headless egui frames for the menu bar.
//!
//! Menu items are reached by clicking the painted menu-button rect, then the
//! painted item rect on a later frame. `egui_kittest` is not used.

#[path = "common/egui_click.rs"]
mod egui_click;

use aw_editor_lib::panel_type::PanelType;
use aw_editor_lib::ui::menu_bar::{
    AlignDirection, DistributeDirection, MenuActionHandler, MenuBar,
};
use aw_editor_lib::viewport::ViewportLayout;
use egui_click::{painted_text, text_rect, UiFrame};
use std::path::PathBuf;

struct MockHandler {
    new_called: bool,
    undo_called: bool,
    group_called: bool,
    selection_count: usize,
    station_name: String,
}

impl MenuActionHandler for MockHandler {
    fn on_new(&mut self) {
        self.new_called = true;
    }
    fn on_new_world_wizard(&mut self) {}
    fn on_open(&mut self) {}
    fn on_save(&mut self) {}
    fn on_save_json(&mut self) {}
    fn on_save_scene(&mut self) {}
    fn on_load_scene(&mut self) {}
    fn on_exit(&mut self) {}

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

    fn get_recent_files(&self) -> Vec<PathBuf> {
        Vec::new()
    }
    fn on_open_recent(&mut self, _path: PathBuf) {}
    fn on_clear_recent(&mut self) {}

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

    fn viewport_layout(&self) -> ViewportLayout {
        ViewportLayout::default()
    }
    fn set_viewport_layout(&mut self, _layout: ViewportLayout) {}

    fn is_docking_enabled(&self) -> bool {
        true
    }
    fn toggle_docking(&mut self) {}
    fn on_apply_layout_preset(&mut self, _preset_name: &str) {}
    fn is_dock_panel_visible(&self, _panel: PanelType) -> bool {
        true
    }
    fn toggle_dock_panel(&mut self, _panel: PanelType) {}

    fn on_open_settings(&mut self) {}

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

    fn on_show_tutorial(&mut self) {}
    fn on_show_about(&mut self) {}

    fn camera_station_names(&self) -> Vec<String> {
        Vec::new()
    }
    fn on_pin_camera_station(&mut self) {}
    fn on_restore_camera_station(&mut self, _index: usize) {}
    fn on_delete_camera_station(&mut self, _index: usize) {}
    fn on_capture_viewport(&mut self) {}
    fn on_capture_station(&mut self, _index: usize) {}
    fn on_capture_all_stations(&mut self) {}
    fn camera_station_name_buffer(&mut self) -> &mut String {
        &mut self.station_name
    }

    fn on_import_blend_scene(&mut self) {}
    fn on_toggle_blueprint_mode(&mut self) {}
    fn is_blueprint_mode(&self) -> bool {
        false
    }
}

fn paint(ctx: &egui::Context, handler: &mut MockHandler) {
    egui::CentralPanel::default().show(ctx, |ui| {
        MenuBar::show(ui, handler);
    });
}

fn menu_item(text: &str, label: &str) -> bool {
    match label {
        "New" => text.starts_with("New") && text.contains("Ctrl+N"),
        "Undo" => text.starts_with("Undo") && text.contains("Ctrl+Z"),
        "Group" => text.starts_with("Group") && text.contains("Ctrl+G"),
        _ => text == label,
    }
}

fn require_item(shapes: &[egui::epaint::ClippedShape], label: &str) -> egui::Rect {
    text_rect(shapes, |text| menu_item(text, label))
        .unwrap_or_else(|| panic!("missing {label:?}. painted: {:?}", painted_text(shapes)))
}

fn click_menu(frame: &mut UiFrame, handler: &mut MockHandler, menu: &str, item: &str) {
    let shapes = frame.paint(|ctx| paint(ctx, handler));
    let menu_pos = require_item(&shapes, menu).left_center() + egui::vec2(2.0, 0.0);
    frame.click(menu_pos, |ctx| paint(ctx, handler));
    // Menu popups are areas. Their item labels show up on the next frame.
    let shapes = frame.paint(|ctx| paint(ctx, handler));
    let item_pos = require_item(&shapes, item).left_center() + egui::vec2(2.0, 0.0);
    frame.click(item_pos, |ctx| paint(ctx, handler));
}

#[test]
fn menu_frame_does_not_fire_new_undo_or_group() {
    let mut handler = MockHandler {
        new_called: false,
        undo_called: false,
        group_called: false,
        selection_count: 2,
        station_name: String::new(),
    };
    let mut frame = UiFrame::new(1280.0, 800.0);
    frame.paint(|ctx| paint(ctx, &mut handler));
    assert!(!handler.new_called);
    assert!(!handler.undo_called);
    assert!(!handler.group_called);
}

#[test]
fn file_new_calls_on_new() {
    let mut handler = MockHandler {
        new_called: false,
        undo_called: false,
        group_called: false,
        selection_count: 0,
        station_name: String::new(),
    };
    let mut frame = UiFrame::new(1280.0, 800.0);
    click_menu(&mut frame, &mut handler, "File", "New");
    assert!(handler.new_called);
    assert!(!handler.undo_called);
    assert!(!handler.group_called);
}

#[test]
fn edit_undo_calls_on_undo() {
    let mut handler = MockHandler {
        new_called: false,
        undo_called: false,
        group_called: false,
        selection_count: 0,
        station_name: String::new(),
    };
    let mut frame = UiFrame::new(1280.0, 800.0);
    click_menu(&mut frame, &mut handler, "Edit", "Undo");
    assert!(handler.undo_called);
    assert!(!handler.new_called);
}

#[test]
fn edit_group_calls_on_group_when_several_entities_are_selected() {
    let mut handler = MockHandler {
        new_called: false,
        undo_called: false,
        group_called: false,
        selection_count: 2,
        station_name: String::new(),
    };
    let mut frame = UiFrame::new(1280.0, 800.0);
    click_menu(&mut frame, &mut handler, "Edit", "Group");
    assert!(handler.group_called);
}

#[test]
fn edit_group_does_not_fire_for_a_single_selection() {
    let mut handler = MockHandler {
        new_called: false,
        undo_called: false,
        group_called: false,
        selection_count: 1,
        station_name: String::new(),
    };
    let mut frame = UiFrame::new(1280.0, 800.0);
    click_menu(&mut frame, &mut handler, "Edit", "Group");
    assert!(!handler.group_called);
}
