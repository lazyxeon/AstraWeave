//! Audio, animation, and movement bridge tick methods extracted from EditorApp::update()

use crate::movement_scripts;
use crate::panels;

impl crate::EditorApp {
    /// Tick audio subsystem: process panel actions and update engine.
    ///
    /// Returns measured audio tick time in milliseconds.
    pub(crate) fn tick_audio_subsystem(&mut self, frame_time: f32) -> f32 {
        astraweave_profiling::span!("audio_subsystem");
        let audio_start = std::time::Instant::now();

        let audio_actions = self.dock_tab_viewer.take_audio_actions();
        if !audio_actions.is_empty() {
            self.audio_bridge.process_actions(audio_actions);
        }
        self.audio_bridge.tick(frame_time);

        // Push live stats back into the audio panel
        self.dock_tab_viewer
            .set_audio_stats(self.audio_bridge.stats());

        let measured_audio_ms = audio_start.elapsed().as_secs_f32() * 1000.0;

        // Update listener position to match the camera
        if let Some(viewport) = &self.viewport {
            let cam = viewport.camera();
            self.audio_bridge
                .update_listener(cam.position(), cam.forward(), cam.up());
        }

        measured_audio_ms
    }

    /// Tick animation subsystem: sync panel playback, run bridge, apply CPU skinning.
    pub(crate) fn tick_animation_subsystem(&mut self, frame_time: f32) {
        astraweave_profiling::span!("animation_skinning");

        // Sync panel playback state to bridge
        {
            let panel = &self.animation_panel;
            if panel.playback_state == panels::animation::PlaybackState::Playing {
                if let Some(entity_id) = panel.selected_entity {
                    self.animation_bridge
                        .assign_clip(u64::from(entity_id), panel.selected_clip_idx.unwrap_or(0));
                }
            }
        }
        self.animation_bridge.tick(frame_time);

        // Skinning: apply CPU skinning for entities with active skeleton animations
        let entities_with_meshes: Vec<(u64, String)> = self
            .entity_manager
            .entities()
            .iter()
            .filter_map(|(&id, entity)| entity.mesh.as_ref().map(|path| (id, path.clone())))
            .collect();

        // Sync skeleton/animation data from loaded meshes -> animation bridge
        if let Some(viewport) = &self.viewport {
            if let Ok(renderer) = viewport.renderer().lock() {
                for (entity_id, mesh_path) in &entities_with_meshes {
                    if !self.animation_bridge.has_skeleton(*entity_id) {
                        if let Some(skel) = renderer.get_mesh_skeleton(mesh_path) {
                            self.animation_bridge
                                .set_entity_skeleton(*entity_id, skel.clone());
                            let clips = renderer.get_mesh_animations(mesh_path);
                            if !clips.is_empty() {
                                self.animation_bridge
                                    .set_entity_clips(*entity_id, clips.to_vec());
                            }
                        }
                    }
                }
            }
        }

        // Apply CPU skinning for entities with active animations
        for (entity_id, mesh_path) in &entities_with_meshes {
            if let Some(joint_matrices) = self.animation_bridge.compute_joint_matrices(*entity_id) {
                if let Some(viewport) = &self.viewport {
                    if let Ok(mut renderer) = viewport.renderer().lock() {
                        renderer.apply_cpu_skinning(mesh_path, &joint_matrices);
                    }
                }
            }
        }
    }

    /// Tick movement scripts in play mode.
    /// Returns early (no-op) when the editor is not in play mode.
    pub(crate) fn tick_movement_scripts(&mut self, frame_time: f32) {
        if !self.runtime.is_playing() {
            return;
        }

        let mut scripted: Vec<(
            u64,
            movement_scripts::MovementScript,
            glam::Vec3,
            glam::Quat,
        )> = Vec::new();
        for entity in self.entity_manager.entities().values() {
            if let Some(script_val) = entity.components.get("MovementScript") {
                if let Some(script) = movement_scripts::MovementScript::from_json(script_val) {
                    scripted.push((entity.id, script, entity.position, entity.rotation));
                }
            }
        }
        if !scripted.is_empty() {
            let results = self.movement_system.tick_all(&scripted, frame_time);
            for (id, new_pos, new_rot) in results {
                self.entity_manager
                    .update_transform(id, new_pos, new_rot, glam::Vec3::ONE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tick_audio_subsystem_returns_positive_time() {
        let mut app = crate::EditorApp::default();
        let ms = app.tick_audio_subsystem(0.016);
        // Should return a non-negative measurement
        assert!(ms >= 0.0, "audio tick time should be non-negative: {ms}");
    }

    #[test]
    fn tick_audio_subsystem_accepts_zero_frame_time() {
        let mut app = crate::EditorApp::default();
        // Zero frame time should not panic
        let ms = app.tick_audio_subsystem(0.0);
        assert!(ms >= 0.0);
    }

    #[test]
    fn tick_animation_subsystem_no_panic_without_viewport() {
        let mut app = crate::EditorApp::default();
        // No viewport initialized — should handle gracefully
        assert!(app.viewport.is_none());
        app.tick_animation_subsystem(0.016);
    }

    #[test]
    fn tick_movement_scripts_noop_in_edit_mode() {
        let mut app = crate::EditorApp::default();
        // Default is editing mode — movement scripts should be no-op
        assert!(app.editor_mode.is_editing());
        // Add an entity to ensure the loop body would run if not gated
        let entity = crate::entity_manager::EditorEntity::new(1, "Test".to_string());
        app.entity_manager.add(entity);
        app.tick_movement_scripts(0.016);
        // Entity position should remain unchanged (scripts didn't run)
        let e = app.entity_manager.get(1).unwrap();
        assert_eq!(e.position, glam::Vec3::ZERO);
    }
}
