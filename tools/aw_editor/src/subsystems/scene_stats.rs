//! Scene statistics gathering extracted from EditorApp::update()

use crate::panels::profiler_panel::GpuMetrics;
use crate::panels::SceneStats;

impl crate::EditorApp {
    /// Gather and update scene statistics for the stats panel.
    ///
    /// Collects triangle counts, vertex counts, draw calls, and memory usage
    /// from the viewport renderer and entity manager. Uses real GPU memory
    /// budget data when available, falling back to estimates otherwise.
    pub(crate) fn update_scene_stats(&mut self) {
        astraweave_profiling::span!("scene_stats");

        let selected_count = self.selection_set.entities.len();
        let scene_entity_count = self
            .scene_state
            .as_ref()
            .map(|s| s.world().entities().len())
            .unwrap_or(0);

        // Pull real data from terrain + scatter renderers
        let (real_triangles, real_vertices, real_draw_calls, scatter_instances, gpu_mem_kb) =
            if let Some(viewport) = &self.viewport {
                if let Ok(renderer) = viewport.renderer().lock() {
                    let terrain_tris = renderer.terrain_triangles();
                    let terrain_indices = renderer.terrain_indices();
                    let scatter_tris = renderer.scatter_triangles();
                    let scatter_verts = renderer.scatter_vertices();
                    let scatter_dc = renderer.scatter_draw_calls() as usize;
                    let scatter_inst = renderer.scatter_instance_count() as usize;

                    // Get real GPU memory usage from budget tracker
                    let (gpu_used, _, _) = renderer.gpu_memory_stats();
                    let gpu_kb = (gpu_used / 1024) as usize;

                    (
                        terrain_tris + scatter_tris + scene_entity_count * 12,
                        terrain_indices + scatter_verts + scene_entity_count * 8,
                        scatter_dc + scene_entity_count + 2, // +2 for terrain+grid
                        scatter_inst,
                        gpu_kb,
                    )
                } else {
                    let est_tris = scene_entity_count * 500;
                    let est_verts = scene_entity_count * 300;
                    (est_tris, est_verts, scene_entity_count, 0, 0)
                }
            } else {
                let est_tris = scene_entity_count * 500;
                let est_verts = scene_entity_count * 300;
                (est_tris, est_verts, scene_entity_count, 0, 0)
            };

        let mesh_count = scene_entity_count + scatter_instances;
        let mesh_memory_kb = (real_vertices * 32 + real_triangles * 12) / 1024;

        // Use real GPU memory if tracked, otherwise estimate
        let texture_count = (scene_entity_count / 5).max(1) + 10;
        let texture_memory_kb = if gpu_mem_kb > 0 {
            gpu_mem_kb
        } else {
            let avg_texture_size_kb = 256; // 512x512 RGBA compressed fallback
            texture_count * avg_texture_size_kb
        };

        // Material and draw call estimates
        let material_count = (scene_entity_count / 3).max(1) + 2;
        let unique_shader_count = 4; // PBR, unlit, terrain, scatter
        let estimated_state_changes = material_count + unique_shader_count;

        let total_memory_kb = if gpu_mem_kb > 0 {
            gpu_mem_kb + mesh_memory_kb // GPU tracked + mesh estimate
        } else {
            scene_entity_count * 2 + mesh_memory_kb + texture_memory_kb // all estimates
        };

        self.scene_stats_panel.update_stats(SceneStats {
            entity_count: scene_entity_count,
            selected_count,
            component_count: scene_entity_count * 3,
            prefab_count: self.prefab_manager.instance_count(),
            undo_stack_size: self.undo_stack.undo_count(),
            redo_stack_size: self.undo_stack.redo_count(),
            memory_estimate_kb: total_memory_kb,
            scene_path: self
                .current_scene_path
                .as_ref()
                .map(|p| p.display().to_string()),
            is_dirty: self.is_dirty,
            mesh_count,
            total_triangles: real_triangles,
            total_vertices: real_vertices,
            mesh_memory_kb,
            texture_count,
            texture_memory_kb,
            max_texture_resolution: (2048, 2048),
            material_count,
            unique_shader_count,
            estimated_draw_calls: real_draw_calls,
            estimated_state_changes,
            performance_warning: None,
        });

        // Push GPU metrics to the profiler panel for real-time monitoring
        let (gpu_used, gpu_budget, _) = if let Some(viewport) = &self.viewport {
            if let Ok(renderer) = viewport.renderer().lock() {
                renderer.gpu_memory_stats()
            } else {
                (0, 0, 0.0)
            }
        } else {
            (0, 0, 0.0)
        };

        self.profiler_panel.push_gpu_metrics(GpuMetrics {
            draw_calls: real_draw_calls as u32,
            triangles: real_triangles as u32,
            vertices: real_vertices as u32,
            gpu_time_ms: 0.0, // Would need GPU timestamp queries
            vram_used_mb: gpu_used as f32 / (1024.0 * 1024.0),
            vram_total_mb: gpu_budget as f32 / (1024.0 * 1024.0),
            textures_bound: texture_count as u32,
            shader_switches: unique_shader_count as u32,
            state_changes: estimated_state_changes as u32,
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn update_scene_stats_no_panic_without_viewport() {
        let mut app = crate::EditorApp::default();
        // No viewport, no scene state — should handle gracefully
        assert!(app.viewport.is_none());
        app.update_scene_stats();
    }

    #[test]
    fn update_scene_stats_with_scene_state() {
        let mut app = crate::EditorApp::default();
        // Create a scene with entities
        let mut world = astraweave_core::World::new();
        world.spawn(
            "Entity1",
            astraweave_core::IVec2 { x: 0, y: 0 },
            astraweave_core::Team { id: 0 },
            0,
            0,
        );
        world.spawn(
            "Entity2",
            astraweave_core::IVec2 { x: 1, y: 1 },
            astraweave_core::Team { id: 0 },
            0,
            0,
        );
        app.scene_state = Some(crate::scene_state::EditorSceneState::new(world));
        app.update_scene_stats();
        // Should not panic and should have processed the 2 entities
    }

    #[test]
    fn update_scene_stats_populates_profiler_gpu_metrics() {
        let mut app = crate::EditorApp::default();
        app.update_scene_stats();
        // Profiler panel should have received GPU metrics (even if zeros)
        // This tests the push_gpu_metrics path doesn't panic
    }
}
