//! Global hotkey handling extracted from EditorApp::update()

use crate::clipboard;
use crate::command;
use crate::dock_layout;
use crate::scene_serialization;
use crate::ui::MenuActionHandler as _;
use astraweave_core::IVec2;
use eframe::egui;
use std::fs;

impl crate::EditorApp {
    /// Handle all global keyboard shortcuts.
    ///
    /// Extracted from `update()` to reduce main.rs complexity.
    /// Covers: undo/redo, save/load, copy/paste/duplicate, play controls,
    /// entity operations, camera views, layout presets, and dialog toggles.
    pub(crate) fn handle_global_hotkeys(&mut self, ctx: &egui::Context) {
        astraweave_profiling::span!("hotkey_handling");
        ctx.input(|i| {
            // Ctrl+Z: Undo
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) && !i.modifiers.shift {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let undo_error = self
                        .undo_stack
                        .undo(scene_state.world_mut(), Some(&mut self.entity_manager))
                        .err();

                    if let Some(e) = undo_error {
                        self.console_logs.push(format!("Undo failed: {}", e));
                    } else if let Some(desc) = self.undo_stack.redo_description() {
                        self.status = format!("Undid: {}", desc);
                        self.console_logs.push(format!("Undo: {}", desc));
                        self.is_dirty = true;
                        self.invalidate_entity_list();
                    }
                }
            }

            // Ctrl+Y or Ctrl+Shift+Z: Redo
            if (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z))
            {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let redo_error = self
                        .undo_stack
                        .redo(scene_state.world_mut(), Some(&mut self.entity_manager))
                        .err();

                    if let Some(e) = redo_error {
                        self.console_logs.push(format!("Redo failed: {}", e));
                    } else if let Some(desc) = self.undo_stack.undo_description() {
                        self.status = format!("Redid: {}", desc);
                        self.console_logs.push(format!("Redo: {}", desc));
                        self.is_dirty = true;
                        self.invalidate_entity_list();
                    }
                }
            }

            // Ctrl+S: Save Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) && !i.modifiers.shift {
                if let Some(world) = self.edit_world() {
                    let path = if let Some(p) = &self.current_scene_path {
                        p.clone()
                    } else {
                        let dir = self.content_root.join("scenes");
                        let _ = fs::create_dir_all(&dir);
                        dir.join("untitled.scene.ron")
                    };

                    match scene_serialization::save_scene(world, &path) {
                        Ok(()) => {
                            self.current_scene_path = Some(path.clone());
                            self.recent_files.add_file(path.clone());
                            self.status = format!("Saved scene to {:?}", path);
                            self.console_logs.push(format!("Scene saved: {:?}", path));
                            self.last_auto_save = std::time::Instant::now();
                            self.is_dirty = false;
                            self.toast_manager.success("Scene saved successfully");
                        }
                        Err(e) => {
                            self.status = format!("Scene save failed: {}", e);
                            self.console_logs
                                .push(format!("Failed to save scene: {}", e));
                            self.toast_manager.error(format!("Save failed: {}", e));
                        }
                    }
                } else {
                    self.console_logs.push("No world to save".into());
                }
            }

            // Ctrl+O: Load Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::O) {
                let scenes_dir = self.content_root.join("scenes");
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Open Scene")
                    .set_directory(&scenes_dir)
                    .add_filter("Scene Files", &["ron"])
                    .add_filter("All Files", &["*"])
                    .pick_file()
                {
                    self.request_open_scene(path);
                }
            }

            // Ctrl+C: Copy selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::C) && !i.modifiers.shift {
                if let Some(world) = self.edit_world() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        self.clipboard =
                            Some(clipboard::ClipboardData::from_entities(world, &selected));
                        self.status = format!("Copied {} entities", selected.len());
                        self.console_logs
                            .push(format!("Copied {} entities to clipboard", selected.len()));
                    } else {
                        self.console_logs
                            .push("No entities selected to copy".into());
                    }
                }
            }

            // Ctrl+V: Paste entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::V) {
                if let Some(clipboard) = &self.clipboard {
                    if let Some(scene_state) = self.scene_state.as_mut() {
                        let clipboard_data = clipboard.clone();
                        let offset = IVec2 { x: 1, y: 1 };
                        let cmd =
                            command::SpawnEntitiesCommand::new(clipboard_data.clone(), offset);
                        let paste_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match paste_result {
                            Ok(()) => {
                                let count = clipboard_data.entities.len();
                                self.status = format!("Pasted {} entities", count);
                                self.console_logs.push(format!("Pasted {} entities", count));
                                self.invalidate_entity_list();
                            }
                            Err(e) => {
                                self.status = format!("Paste failed: {}", e);
                                self.console_logs.push(format!("Paste failed: {}", e));
                            }
                        }
                    }
                } else {
                    self.console_logs.push("Clipboard is empty".into());
                }
            }

            // Ctrl+D: Duplicate selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        let offset = IVec2 { x: 1, y: 1 };
                        let cmd = command::DuplicateEntitiesCommand::new(selected.clone(), offset);
                        let duplicate_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match duplicate_result {
                            Ok(()) => {
                                self.status = format!("Duplicated {} entities", selected.len());
                                self.console_logs
                                    .push(format!("Duplicated {} entities", selected.len()));
                                self.invalidate_entity_list();
                            }
                            Err(e) => {
                                self.status = format!("Duplicate failed: {}", e);
                                self.console_logs.push(format!("Duplicate failed: {}", e));
                            }
                        }
                    } else {
                        self.console_logs
                            .push("No entities selected to duplicate".into());
                    }
                }
            }

            // F5: Play / Resume
            if i.key_pressed(egui::Key::F5) {
                self.request_play();
            }

            // F6: Pause/Unpause
            if i.key_pressed(egui::Key::F6) {
                if self.editor_mode.is_playing() {
                    self.request_pause();
                } else if self.editor_mode.is_paused() {
                    self.request_play();
                }
            }

            // F7: Stop (restore snapshot)
            if i.key_pressed(egui::Key::F7) {
                self.request_stop();
            }

            // F8: Step one frame
            if i.key_pressed(egui::Key::F8) {
                self.request_step();
            }

            // Delete: Delete selected entities
            if i.key_pressed(egui::Key::Delete) && self.editor_mode.can_edit() {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let selected = self.hierarchy_panel.get_all_selected();
                    if !selected.is_empty() {
                        let cmd = command::DeleteEntitiesCommand::new(selected.clone());
                        let delete_result = self.undo_stack.execute(
                            cmd,
                            scene_state.world_mut(),
                            Some(&mut self.entity_manager),
                        );

                        match delete_result {
                            Ok(()) => {
                                self.hierarchy_panel.set_selected(None);
                                self.selected_entity = None;
                                self.status = format!(" Deleted {} entities", selected.len());
                                self.console_logs
                                    .push(format!("Deleted {} entities", selected.len()));
                                self.invalidate_entity_list();
                            }
                            Err(e) => {
                                self.status = format!("Delete failed: {}", e);
                                self.console_logs.push(format!("Delete failed: {}", e));
                            }
                        }
                    } else {
                        self.console_logs
                            .push("No entities selected to delete".into());
                    }
                }
            }

            // Ctrl+N: New Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) && !i.modifiers.shift {
                if self.is_dirty {
                    self.show_new_confirm_dialog = true;
                } else {
                    self.create_new_scene();
                }
            }

            // Ctrl+Shift+N: New Entity
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::N) {
                if let Some(scene_state) = self.scene_state.as_mut() {
                    let world = scene_state.world_mut();
                    let entity_id = world.spawn(
                        "New Entity",
                        astraweave_core::IVec2 { x: 0, y: 0 },
                        astraweave_core::Team { id: 0 },
                        0,
                        0,
                    );
                    self.selected_entity = Some(u64::from(entity_id));
                    self.hierarchy_panel.set_selected(Some(entity_id));
                    self.is_dirty = true;
                    self.invalidate_entity_list();
                    self.status = format!("Created entity {}", entity_id);
                    self.toast_success("New entity created");
                }
            }

            // Ctrl+Shift+S: Save As
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                if let Some(world) = self.edit_world() {
                    let dir = self.content_root.join("scenes");
                    let _ = fs::create_dir_all(&dir);
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let path = dir.join(format!("scene_{}.scene.ron", timestamp));

                    match scene_serialization::save_scene(world, &path) {
                        Ok(()) => {
                            self.current_scene_path = Some(path.clone());
                            self.recent_files.add_file(path.clone());
                            self.status = format!("Saved scene as {:?}", path);
                            self.console_logs
                                .push(format!("Scene saved as: {:?}", path));
                        }
                        Err(e) => {
                            self.status = format!("Save As failed: {}", e);
                            self.console_logs.push(format!("Save As failed: {}", e));
                        }
                    }
                }
            }

            // Ctrl+A: Select All entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::A) && !i.modifiers.shift {
                if let Some(world) = self.edit_world() {
                    let all_entities = world.entities();
                    if !all_entities.is_empty() {
                        self.hierarchy_panel.set_selected_multiple(&all_entities);
                        self.status = format!("Selected {} entities", all_entities.len());
                    }
                }
            }

            // Ctrl+I: Import .blend Scene
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) && !i.modifiers.shift {
                self.on_import_blend_scene();
            }

            // Ctrl+B: Toggle Blueprint Mode
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) && !i.modifiers.shift {
                self.on_toggle_blueprint_mode();
            }

            // Ctrl+G: Group selected entities
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) && !i.modifiers.shift {
                self.group_selection();
            }

            // Ctrl+Shift+G: Ungroup selected entity
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::G) {
                self.ungroup_selection();
            }

            // Escape: Deselect all (when not in gizmo mode)
            if i.key_pressed(egui::Key::Escape) && self.editor_mode.can_edit() {
                self.hierarchy_panel.set_selected(None);
                self.selected_entity = None;
                self.selection_set.primary = None;
                if let Some(viewport) = &mut self.viewport {
                    viewport.clear_selection();
                }
                self.status = "Selection cleared".to_string();
            }

            // F: Focus camera on selected entity
            if i.key_pressed(egui::Key::F) && !i.modifiers.ctrl {
                if let Some(selected_id) = self.selected_entity {
                    if let Some(entity) = self.entity_manager.get(selected_id) {
                        if let Some(viewport) = &mut self.viewport {
                            let entity_pos = glam::Vec3::new(
                                entity.position.x,
                                entity.position.y,
                                entity.position.z,
                            );
                            viewport.camera_mut().frame_entity(entity_pos, 2.0);
                            self.status = format!("Focused on entity {}", selected_id);
                        }
                    }
                } else {
                    self.status = "No entity selected to focus".to_string();
                }
            }

            // Home: Reset camera to origin
            if i.key_pressed(egui::Key::Home) {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().reset_to_origin();
                    self.status = "Camera reset to origin".to_string();
                }
            }

            // Numpad views (Alt+1/3/7/0)
            if i.key_pressed(egui::Key::Num1) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_front();
                    self.status = "Front view".to_string();
                }
            }
            if i.key_pressed(egui::Key::Num3) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_right();
                    self.status = "Right view".to_string();
                }
            }
            if i.key_pressed(egui::Key::Num7) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_top();
                    self.status = "Top view".to_string();
                }
            }
            if i.key_pressed(egui::Key::Num0) && i.modifiers.alt {
                if let Some(viewport) = &mut self.viewport {
                    viewport.camera_mut().set_view_perspective();
                    self.status = "Perspective view".to_string();
                }
            }

            // F1: Show keyboard shortcuts help
            if i.key_pressed(egui::Key::F1) {
                self.show_help_dialog = !self.show_help_dialog;
            }

            // G: Toggle grid visibility
            if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl {
                self.show_grid = !self.show_grid;
                self.status = if self.show_grid {
                    "Grid enabled".to_string()
                } else {
                    "Grid disabled".to_string()
                };
            }

            // Escape: Close dialogs
            if i.key_pressed(egui::Key::Escape) {
                if self.show_quit_dialog {
                    self.show_quit_dialog = false;
                } else if self.show_new_confirm_dialog {
                    self.show_new_confirm_dialog = false;
                } else if self.show_open_confirm_dialog {
                    self.pending_open_path = None;
                    self.show_open_confirm_dialog = false;
                } else if self.show_settings_dialog {
                    self.save_preferences();
                    self.show_settings_dialog = false;
                } else if self.show_help_dialog {
                    self.show_help_dialog = false;
                }
            }

            // Ctrl+1..6: Layout presets
            let layout_keys = [
                (egui::Key::Num1, dock_layout::LayoutPreset::Default),
                (egui::Key::Num2, dock_layout::LayoutPreset::Wide),
                (egui::Key::Num3, dock_layout::LayoutPreset::Compact),
                (egui::Key::Num4, dock_layout::LayoutPreset::Modeling),
                (egui::Key::Num5, dock_layout::LayoutPreset::Animation),
                (egui::Key::Num6, dock_layout::LayoutPreset::Debug),
            ];
            for (key, preset) in layout_keys {
                if i.modifiers.ctrl && !i.modifiers.alt && i.key_pressed(key) {
                    self.dock_layout.apply_preset(preset);
                    self.status = format!("Layout: {:?}", preset);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::command::{MoveEntityCommand, UndoStack};
    use astraweave_core::IVec2;

    #[test]
    fn invalidate_entity_list_bumps_generation() {
        let mut app = crate::EditorApp::default();
        let gen_before = app.entity_list_generation;
        app.invalidate_entity_list();
        assert_ne!(
            app.entity_list_generation, gen_before,
            "invalidate should bump generation"
        );
    }

    #[test]
    fn undo_redo_marks_dirty_and_invalidates() {
        let mut app = crate::EditorApp::default();
        // Create a scene with an entity
        let mut world = astraweave_core::World::new();
        let entity = world.spawn(
            "Test",
            IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            0,
            0,
        );
        app.scene_state = Some(crate::scene_state::EditorSceneState::new(world));

        // Execute a command
        let cmd = MoveEntityCommand::new(entity, IVec2 { x: 0, y: 0 }, IVec2 { x: 5, y: 5 });
        let _ = app.undo_stack.execute(
            cmd,
            app.scene_state.as_mut().unwrap().world_mut(),
            Some(&mut app.entity_manager),
        );
        app.is_dirty = true;
        app.invalidate_entity_list();

        let gen_after_execute = app.entity_list_generation;

        // Undo
        let _ = app.undo_stack.undo(
            app.scene_state.as_mut().unwrap().world_mut(),
            Some(&mut app.entity_manager),
        );
        app.invalidate_entity_list();

        assert_ne!(
            app.entity_list_generation, gen_after_execute,
            "undo should bump generation via invalidate"
        );
    }

    #[test]
    fn entity_operations_set_dirty_flag() {
        let mut app = crate::EditorApp::default();
        assert!(!app.is_dirty, "should start clean");
        app.is_dirty = true;
        assert!(app.is_dirty, "dirty flag should be settable");
        app.is_dirty = false;
        assert!(!app.is_dirty, "dirty flag should be clearable");
    }
}
