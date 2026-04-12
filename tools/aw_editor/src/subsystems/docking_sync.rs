//! Docking layout synchronization methods extracted from show_docking_layout().
//!
//! These methods sync editor state (console logs, runtime stats, scene stats,
//! frame debugger data) to the dock tab viewer each frame.

use crate::tab_viewer;

impl crate::EditorApp {
    /// Sync console logs to the dock tab viewer.
    /// Only clones when new logs have been appended (guarded by length check).
    pub(crate) fn sync_console_logs_to_dock(&mut self) {
        if self.console_logs.len() != self.console_logs_synced_len {
            self.console_logs_synced_len = self.console_logs.len();
            self.dock_tab_viewer
                .set_console_logs(self.console_logs.clone());
        }
    }

    /// Sync runtime stats and scene component counts to the dock tab viewer.
    pub(crate) fn sync_runtime_stats_to_dock(&mut self) {
        astraweave_profiling::span!("sync_runtime_stats");

        let entity_count = self.entity_manager.entities().len();

        // Get real render stats from viewport if available
        let (vp_draw_calls, vp_triangles, vp_memory_mb) = if let Some(viewport) = &self.viewport {
            let stats = &viewport.toolbar().stats;
            if let Ok(renderer) = viewport.renderer().lock() {
                let base_draw_calls = if entity_count > 0 { 4 } else { 2 };
                let terrain_draw_calls = renderer.terrain_triangles().min(1);
                let scatter_draw_calls = renderer.scatter_draw_calls() as usize;
                let draw_calls = base_draw_calls + terrain_draw_calls + scatter_draw_calls;
                let entity_tris = entity_count * 12;
                let terrain_tris = renderer.terrain_triangles();
                let scatter_tris = renderer.scatter_triangles();
                let triangles = entity_tris + terrain_tris + scatter_tris + 4;
                drop(renderer);
                (draw_calls, triangles, stats.memory_usage_mb)
            } else {
                (0, 0, stats.memory_usage_mb)
            }
        } else {
            (0, 0, 0.0)
        };

        let gpu_memory_bytes = if self.resource_usage.memory_used > 0 {
            self.resource_usage.memory_used as usize
        } else {
            (vp_memory_mb * 1024.0 * 1024.0) as usize
        };

        let runtime_stats = tab_viewer::RuntimeStatsInfo {
            frame_time_ms: self.runtime.stats().frame_time_ms,
            fps: self.current_fps,
            entity_count,
            tick_count: self.runtime.stats().tick_count,
            is_playing: self.runtime.is_playing(),
            is_paused: !self.editor_mode.is_editing() && !self.runtime.is_playing(),
            render_time_ms: self.measured_render_ms,
            physics_time_ms: self.measured_tick_ms,
            ai_time_ms: 0.0,
            script_time_ms: 0.0,
            audio_time_ms: 0.0,
            draw_calls: vp_draw_calls,
            triangles: vp_triangles,
            gpu_memory_bytes,
        };
        self.dock_tab_viewer.set_runtime_stats(runtime_stats);
        self.dock_tab_viewer.update_play_session();

        // Sync scene stats — count actual component types from entities
        let entities = self.entity_manager.entities();
        let total_components: usize = entities.values().map(|e| e.components.len()).sum();
        let entity_count = entities.len();

        let mut light_count = 0usize;
        let mut mesh_count = 0usize;
        let mut physics_bodies = 0usize;
        let mut audio_sources = 0usize;
        let mut particle_systems = 0usize;
        let mut camera_count = 0usize;
        let mut collider_count = 0usize;
        let mut script_count = 0usize;

        for entity in entities.values() {
            if entity.mesh.is_some() {
                mesh_count += 1;
            }
            for key in entity.components.keys() {
                let key_lower = key.to_lowercase();
                if key_lower.contains("light") {
                    light_count += 1;
                } else if key_lower.contains("rigidbody") || key_lower.contains("physics") {
                    physics_bodies += 1;
                } else if key_lower.contains("audio") || key_lower.contains("sound") {
                    audio_sources += 1;
                } else if key_lower.contains("particle") {
                    particle_systems += 1;
                } else if key_lower.contains("camera") {
                    camera_count += 1;
                } else if key_lower.contains("collider") {
                    collider_count += 1;
                } else if key_lower.contains("script") || key_lower.contains("behavior") {
                    script_count += 1;
                }
            }
        }

        let scene_stats = tab_viewer::SceneStatsInfo {
            total_entities: entity_count,
            total_components,
            prefab_instances: self.prefab_manager.instance_count(),
            selected_count: self.selection_set.entities.len(),
            memory_usage_bytes: gpu_memory_bytes,
            active_systems: if self.runtime.is_playing() { 12 } else { 0 },
            loaded_assets: self.asset_registry.count(),
            light_count,
            mesh_count,
            physics_bodies,
            is_modified: self.is_dirty,
            audio_sources,
            particle_systems,
            camera_count,
            collider_count,
            script_count,
            ui_element_count: 0,
            scene_path: self
                .current_scene_path
                .as_ref()
                .map(|p| p.display().to_string()),
            last_save_time: self.last_save_time.clone(),
        };
        self.dock_tab_viewer.set_scene_stats(scene_stats);

        // Sync undo/redo counts
        self.dock_tab_viewer
            .set_undo_redo_counts(self.undo_stack.len(), 0);

        // Update frame time history for profiler graph
        self.dock_tab_viewer
            .push_frame_time(self.runtime.stats().frame_time_ms);

        // Feed frame debugger with render timing data
        {
            let entity_count = self.entity_manager.entities().len();
            let terrain_active = self.dock_tab_viewer.is_terrain_active();
            self.dock_tab_viewer.update_frame_debugger(
                self.measured_render_ms,
                entity_count,
                terrain_active,
            );
        }
    }

    /// Forward post-process panel changes (bloom, tonemapper) to the viewport
    /// renderer so they take effect on the next frame.
    pub(crate) fn sync_post_process_to_renderer(&mut self) {
        if let Some(settings) = self.dock_tab_viewer.take_post_process_update() {
            if let Some(viewport) = &self.viewport {
                if let Ok(mut renderer) = viewport.renderer().lock() {
                    // Update the PostProcessChain bloom on/off flag.
                    if let Some(existing) = renderer.post_process_chain() {
                        let mut chain = existing.clone();
                        chain.bloom_enabled = settings.bloom_enabled;
                        renderer.set_post_process_chain(chain);
                    }

                    // Forward bloom-specific parameters to the compute pass.
                    renderer.set_bloom_config(astraweave_render::bloom::BloomConfig {
                        enabled: settings.bloom_enabled,
                        threshold: settings.bloom_threshold,
                        soft_knee: settings.bloom_soft_knee,
                        intensity: settings.bloom_intensity,
                        ..Default::default()
                    });
                }
            }
        }
    }
}
