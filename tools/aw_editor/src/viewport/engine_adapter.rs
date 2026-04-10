use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

use super::camera::OrbitCamera;
use super::types::{
    ScatterPlacement, TerrainFogParams, TerrainLightingParams, TerrainVertex, WaterStyle,
};

/// Render mode for the editor viewport.
///
/// Controls whether the viewport uses the full engine PBR pipeline or a
/// lightweight cube-based preview for fast iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Full engine PBR rendering (default): sky, shadows, PBR materials,
    /// water, weather particles, post-processing via `astraweave-render`.
    EnginePBR,
    /// Fast preview: cube placeholders per entity, simple gradient skybox,
    /// no PBR materials. Faster on weak GPUs or very large scenes.
    FastPreview,
}

impl Default for RenderMode {
    fn default() -> Self {
        Self::EnginePBR
    }
}

/// Editor rendering quality preset.
///
/// Controls shadow quality and post-processing complexity to balance
/// visual fidelity vs. frame time in the editor viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorQualityPreset {
    /// Full game-quality rendering: 2 CSM cascades at full resolution,
    /// all post-processing effects enabled. Use for final preview.
    GameQuality,
    /// Editor default: reduced shadow quality (smaller PCF, narrower cascades),
    /// only SSAO + Bloom + Tonemap post-processing. Good balance for editing.
    EditorDefault,
    /// Terrain-optimised: 2-cascade shadows, SSAO, bloom + tonemap.
    /// Applied automatically when terrain is loaded. Strikes a balance
    /// between visual fidelity (grounded shadows, AO) and performance.
    EditorTerrain,
    /// Minimal: shadows disabled, tonemap only. Maximum performance for
    /// large scenes or weak GPUs.
    Minimal,
}

impl Default for EditorQualityPreset {
    fn default() -> Self {
        Self::EditorDefault
    }
}

pub struct EngineRenderAdapter {
    renderer: astraweave_render::Renderer,
    initialized: bool,
    /// Tracks which terrain chunk model names we've uploaded, so we can
    /// clear stale chunks on re-upload.
    terrain_model_names: Vec<String>,
    /// Tracks scatter model names for cleanup.
    scatter_model_names: Vec<String>,
    /// Total terrain triangles across all uploaded chunks.
    terrain_total_triangles: usize,
    /// Total terrain indices across all uploaded chunks.
    terrain_total_indices: usize,
    /// Total scatter placements last uploaded.
    scatter_placement_count: usize,
    /// Number of unique scatter draw groups (one draw call per mesh type).
    scatter_draw_call_count: u32,
    /// Whether weather effects are currently active.
    weather_active: bool,
    /// Whether water rendering is enabled.
    water_enabled: bool,
    /// Cached index buffers per terrain chunk for incremental vertex updates.
    /// Brush strokes only change heights/normals, not topology.
    terrain_cached_indices: Vec<Vec<u32>>,
    /// Current editor quality preset (shadows + post-processing).
    quality_preset: EditorQualityPreset,
    /// Cached entity count + selection for dirty-skip in feed_entities
    cached_entity_feed_count: usize,
    /// Cached selected entity set for feed_entities dirty check
    cached_entity_feed_selected: Vec<astraweave_core::Entity>,
    /// Cached mesh map length for feed_entities dirty check
    cached_entity_feed_mesh_count: usize,
}

impl EngineRenderAdapter {
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let device_owned = (*device).clone();
        let queue_owned = (*queue).clone();

        let renderer =
            astraweave_render::Renderer::new_from_device(device_owned, queue_owned, None, config)
                .await
                .context("Failed to create engine renderer")?;

        let mut adapter = Self {
            renderer,
            initialized: true,
            terrain_model_names: Vec::new(),
            scatter_model_names: Vec::new(),
            terrain_total_triangles: 0,
            terrain_total_indices: 0,
            scatter_placement_count: 0,
            scatter_draw_call_count: 0,
            weather_active: false,
            water_enabled: false,
            terrain_cached_indices: Vec::new(),
            quality_preset: EditorQualityPreset::default(),
            cached_entity_feed_count: usize::MAX, // force first rebuild
            cached_entity_feed_selected: Vec::new(),
            cached_entity_feed_mesh_count: usize::MAX,
        };
        adapter.apply_quality_preset(EditorQualityPreset::EditorDefault);
        Ok(adapter)
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn update_camera(&mut self, camera: &OrbitCamera) {
        // Pass the OrbitCamera's own view/proj matrices directly to the renderer.
        // This avoids yaw/pitch conversion issues between the orbit camera and
        // the engine camera's direction conventions.
        self.renderer.update_camera_matrices(
            camera.view_matrix(),
            camera.projection_matrix(),
            camera.position(),
            camera.near,
            camera.far,
            camera.fov.to_radians(),
            camera.aspect,
        );
    }

    pub fn render_to_texture(
        &mut self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        self.renderer
            .draw_into(target, encoder)
            .context("Engine draw_into failed")
    }

    pub fn renderer(&self) -> &astraweave_render::Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut astraweave_render::Renderer {
        &mut self.renderer
    }

    /// Get the current quality preset.
    pub fn quality_preset(&self) -> EditorQualityPreset {
        self.quality_preset
    }

    /// Get GPU memory usage statistics from the budget tracker.
    /// Returns (total_used_bytes, total_budget_bytes, usage_percentage).
    pub fn gpu_memory_stats(&self) -> (u64, u64, f32) {
        let budget = self.renderer.gpu_memory_budget();
        let usage_pct = budget.usage_percentage();
        if usage_pct > 80.0 {
            tracing::warn!(target: "aw_editor::viewport", "GPU memory high: {:.1}% ({} MB used)", usage_pct, budget.total_usage() / (1024 * 1024));
        }
        (
            budget.total_usage(),
            2 * 1024 * 1024 * 1024, // 2GB default
            usage_pct,
        )
    }

    /// Get per-category GPU memory snapshot.
    /// Returns Vec of (category, current_bytes, hard_limit_bytes).
    pub fn gpu_memory_snapshot(&self) -> Vec<(astraweave_render::MemoryCategory, u64, u64)> {
        self.renderer.gpu_memory_budget().snapshot()
    }

    /// Apply an editor quality preset, configuring shadows and post-processing.
    ///
    /// - `GameQuality`: Full shadows + all post-processing (for final preview)
    /// - `EditorDefault`: Reduced shadows + SSAO/Bloom/Tonemap only (balanced)
    /// - `Minimal`: No shadows + tonemap only (maximum performance)
    pub fn apply_quality_preset(&mut self, preset: EditorQualityPreset) {
        tracing::info!(target: "aw_editor::viewport", "Quality preset changed to: {:?}", preset);
        self.quality_preset = preset;

        match preset {
            EditorQualityPreset::GameQuality => {
                // Full game-quality shadows
                self.renderer.set_shadows_enabled(true);
                self.renderer.set_shadow_filter(2.0, 0.005, 1.5);
                self.renderer.set_cascade_extents(40.0, 120.0);
                self.renderer.set_cascade_lambda(0.75);

                // Full post-processing chain
                // NOTE: ssao/ssr disabled until proper compute shaders exist;
                // current passes reuse the tonemap pipeline causing double-tonemap.
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false,
                    ssr_enabled: false,
                    bloom_enabled: true,
                    taa_enabled: true,
                    dof_enabled: false, // DoF off by default even in game quality
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::EditorDefault => {
                // Shadows disabled in editor default for terrain performance
                // (each shadow cascade costs ~2-3ms; with 121-chunk terrain this dominates)
                self.renderer.set_shadows_enabled(false);

                // Editor post-processing: only essential effects
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false, // SSAO disabled for terrain perf
                    ssr_enabled: false,
                    bloom_enabled: true,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::EditorTerrain => {
                // Terrain-optimised: shadows enabled for surface definition.
                // The ground plane overwrite bug that caused the camera-
                // following shadow artifact has been fixed — draw_into() now
                // preserves the terrain ground plane position set by
                // set_terrain_ground_plane().
                self.renderer.set_shadows_enabled(true);

                // NOTE: ssao_enabled and ssr_enabled are set to false because
                // the renderer's SSR/SSAO passes currently reuse the tonemap
                // pipeline, which double-tonemaps the scene (dark/muddy output).
                // Re-enable when proper compute-based SSAO/SSR shaders exist.
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false,
                    ssr_enabled: false,
                    bloom_enabled: true,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: true,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
            EditorQualityPreset::Minimal => {
                // Shadows disabled
                self.renderer.set_shadows_enabled(false);

                // Minimal post-processing: tonemap only
                let chain = astraweave_render::hdr_pipeline::PostProcessChain {
                    ssao_enabled: false,
                    ssr_enabled: false,
                    bloom_enabled: false,
                    taa_enabled: false,
                    dof_enabled: false,
                    motion_blur_enabled: false,
                    color_grading_enabled: false,
                    tonemap_operator: astraweave_render::hdr_pipeline::TonemapOperator::Aces,
                };
                self.renderer.set_post_process_chain(chain);
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.renderer.resize(width, height);
        }
    }

    pub fn load_gltf_model(&mut self, name: impl Into<String>, path: &Path) -> Result<()> {
        use astraweave_render::{mesh_gltf, Instance};

        let name = name.into();
        tracing::info!("Loading glTF model '{}' from: {}", name, path.display());

        let opts = mesh_gltf::GltfOptions::default();
        let cpu_meshes = mesh_gltf::load_gltf(path, &opts)
            .with_context(|| format!("Failed to load glTF: {}", path.display()))?;

        if cpu_meshes.is_empty() {
            anyhow::bail!("glTF file contains no meshes: {}", path.display());
        }

        tracing::info!(
            "Loaded {} mesh(es), first mesh has {} vertices, {} indices",
            cpu_meshes.len(),
            cpu_meshes[0].vertices.len(),
            cpu_meshes[0].indices.len()
        );

        let mesh = self.renderer.create_mesh_from_cpu_mesh(&cpu_meshes[0]);
        let instance =
            Instance::from_pos_scale_color(glam::Vec3::ZERO, glam::Vec3::ONE, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.add_model(&name, mesh, &[instance]);
        tracing::info!("Model '{}' added to renderer", name);
        Ok(())
    }

    /// Feed World entities to the engine renderer as named models.
    ///
    /// Iterates all entities in the World, groups them by mesh path, and
    /// updates the engine's model list. Entities without a mesh use the
    /// engine's built-in cube primitive. Selected entities get an orange
    /// tint for highlighting.
    pub fn feed_entities(
        &mut self,
        world: &astraweave_core::World,
        entity_meshes: &std::collections::HashMap<astraweave_core::Entity, String>,
        selected_entities: &[astraweave_core::Entity],
    ) {
        // Skip rebuild when nothing changed (entity count, selection, mesh assignments)
        let entity_count = world.entities().len();
        if entity_count == self.cached_entity_feed_count
            && entity_meshes.len() == self.cached_entity_feed_mesh_count
            && selected_entities == self.cached_entity_feed_selected.as_slice()
        {
            return;
        }
        self.cached_entity_feed_count = entity_count;
        self.cached_entity_feed_mesh_count = entity_meshes.len();
        self.cached_entity_feed_selected = selected_entities.to_vec();

        use astraweave_render::Instance;
        use std::collections::HashMap;

        // Group instances by mesh path (None = default cube)
        let mut mesh_groups: HashMap<Option<String>, Vec<Instance>> = HashMap::new();

        for entity in world.entities() {
            if let Some(pose) = world.pose(entity) {
                let x = if pose.use_float_pos {
                    pose.float_x
                } else {
                    pose.pos.x as f32
                };
                let z = if pose.use_float_pos {
                    pose.float_z
                } else {
                    pose.pos.y as f32
                };
                let position = glam::Vec3::new(x, pose.height, z);
                let scale = glam::Vec3::new(pose.scale, pose.scale_y, pose.scale_z);
                let rotation = glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    pose.rotation_x,
                    pose.rotation,
                    pose.rotation_z,
                );
                let transform =
                    glam::Mat4::from_scale_rotation_translation(scale, rotation, position);

                let is_selected = selected_entities.contains(&entity);
                let color = if is_selected {
                    [1.0, 0.6, 0.2, 1.0] // Orange highlight
                } else if let Some(team) = world.team(entity) {
                    match team.id {
                        0 => [0.2, 0.8, 0.3, 1.0],
                        1 => [0.3, 0.6, 1.0, 1.0],
                        2 => [1.0, 0.3, 0.2, 1.0],
                        _ => [0.6, 0.6, 0.7, 1.0],
                    }
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                };

                let instance = Instance {
                    transform,
                    color,
                    material_id: 0,
                };

                let mesh_key = entity_meshes.get(&entity).cloned();
                mesh_groups.entry(mesh_key).or_default().push(instance);
            }
        }

        // Determine which entity model names are still active this frame
        let mut active_names: std::collections::HashSet<String> =
            std::collections::HashSet::with_capacity(mesh_groups.len());

        // Add each group as a named model
        for (mesh_key, instances) in &mesh_groups {
            let model_name = match mesh_key {
                Some(path) => format!("entity_mesh_{}", path.replace(['/', '\\', '.'], "_")),
                None => "entity_default_cubes".to_string(),
            };
            active_names.insert(model_name.clone());

            // Fast path: model already exists → just update the instance buffer
            // (reuses the existing mesh GPU buffers, no disk I/O)
            if self.renderer.update_model_instances(&model_name, instances) {
                continue;
            }

            // Slow path: first time seeing this model → load mesh and create GPU resources
            let mesh = match mesh_key {
                Some(path) => {
                    let opts = astraweave_render::mesh_gltf::GltfOptions::default();
                    match astraweave_render::mesh_gltf::load_gltf(Path::new(path), &opts) {
                        Ok(cpu_meshes) if !cpu_meshes.is_empty() => {
                            self.renderer.create_mesh_from_cpu_mesh(&cpu_meshes[0])
                        }
                        _ => {
                            // Fallback: use simple cube arrays
                            self.renderer.create_mesh_from_arrays(
                                &CUBE_POSITIONS,
                                &CUBE_NORMALS,
                                &CUBE_INDICES,
                            )
                        }
                    }
                }
                None => self.renderer.create_mesh_from_arrays(
                    &CUBE_POSITIONS,
                    &CUBE_NORMALS,
                    &CUBE_INDICES,
                ),
            };
            self.renderer.add_model(&model_name, mesh, instances);
        }

        // Remove entity models that no longer have any instances
        let stale_names: Vec<String> = self
            .renderer
            .model_names_with_prefix("entity_")
            .into_iter()
            .filter(|n| !active_names.contains(n))
            .collect();
        for name in &stale_names {
            self.renderer.clear_model(name);
        }
    }

    /// Invalidate the feed_entities cache so the next call rebuilds all entity transforms.
    /// Call when entity transforms change (gizmo drag, undo, paste, etc.)
    pub fn invalidate_entity_cache(&mut self) {
        self.cached_entity_feed_count = usize::MAX;
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.renderer.has_model(name)
    }

    pub fn clear_model(&mut self, name: &str) {
        self.renderer.clear_model(name);
    }

    /// Set material parameters for the current model
    pub fn set_material_params(&mut self, base_color: [f32; 4], metallic: f32, roughness: f32) {
        self.renderer
            .set_material_params(base_color, metallic, roughness);
        tracing::debug!(
            "Material params set: color={:?}, metallic={}, roughness={}",
            base_color,
            metallic,
            roughness
        );
    }

    /// Get model count
    pub fn model_count(&self) -> usize {
        self.renderer.model_count()
    }

    /// List all loaded model names
    pub fn model_names(&self) -> Vec<String> {
        self.renderer.model_names()
    }

    /// Get current time of day (0.0 - 24.0 game hours)
    pub fn get_time_of_day(&self) -> f32 {
        self.renderer.time_of_day().current_time
    }

    /// Set time of day (0.0 - 24.0 game hours)
    pub fn set_time_of_day(&mut self, hour: f32) {
        let time = self.renderer.time_of_day_mut();
        time.current_time = hour.clamp(0.0, 24.0);
        tracing::debug!("Time of day set to: {:.1}h", time.current_time);
    }

    /// Get time scale (1.0 = real time, 60.0 = 1 real minute = 1 game hour)
    pub fn get_time_scale(&self) -> f32 {
        self.renderer.time_of_day().time_scale
    }

    /// Set time scale
    pub fn set_time_scale(&mut self, scale: f32) {
        let time = self.renderer.time_of_day_mut();
        time.time_scale = scale.max(0.0);
        tracing::debug!("Time scale set to: {:.1}x", time.time_scale);
    }

    /// Check if it's currently daytime
    pub fn is_daytime(&self) -> bool {
        self.renderer.time_of_day().is_day()
    }

    /// Get current light direction
    pub fn get_light_direction(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_light_direction()
    }

    /// Get current light color
    pub fn get_light_color(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_light_color()
    }

    /// Get sun position
    pub fn get_sun_position(&self) -> glam::Vec3 {
        self.renderer.time_of_day().get_sun_position()
    }

    /// Get time-of-day period description
    pub fn get_time_period(&self) -> &'static str {
        let time = self.renderer.time_of_day();
        if time.is_night() {
            "Night"
        } else if time.is_twilight() {
            "Twilight"
        } else {
            "Day"
        }
    }

    /// Check if shadows are enabled
    pub fn shadows_enabled(&self) -> bool {
        self.renderer.shadows_enabled()
    }

    /// Enable or disable shadows
    pub fn set_shadows_enabled(&mut self, enabled: bool) {
        self.renderer.set_shadows_enabled(enabled);
        tracing::debug!("Shadows enabled: {}", enabled);
    }

    // ── Terrain chunk feeding ───────────────────────────────────────────

    /// Upload terrain chunks to the engine renderer as named models.
    ///
    /// Each chunk is uploaded as a separate model named `"terrain_chunk_N"`.
    /// Previous terrain data is cleared before uploading fresh chunks.
    pub fn upload_terrain_chunks(&mut self, chunks: &[(Vec<TerrainVertex>, Vec<u32>)]) {
        // Clear previous terrain models
        for name in self.terrain_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }

        // Cache indices for incremental brush updates (topology doesn't change)
        self.terrain_cached_indices = chunks.iter().map(|(_, idx)| idx.clone()).collect();

        let total_verts: usize = chunks.iter().map(|(v, _)| v.len()).sum();
        let total_indices: usize = chunks.iter().map(|(_, i)| i.len()).sum();

        if total_verts == 0 || total_indices == 0 {
            return;
        }

        // ── Convert each chunk and compute per-chunk AABB ──────────────
        struct ConvertedChunk {
            positions: Vec<[f32; 3]>,
            normals: Vec<[f32; 3]>,
            tangents: Vec<[f32; 4]>,
            uvs: Vec<[f32; 2]>,
            biome_tint: [f32; 4],
            aabb_min: [f32; 3],
            aabb_max: [f32; 3],
            indices: Vec<u32>,
        }

        let converted: Vec<ConvertedChunk> = chunks
            .iter()
            .filter(|(v, i)| !v.is_empty() && !i.is_empty())
            .map(|(vertices, indices)| {
                let (positions, normals, tangents, uvs, biome_tint, aabb_min, aabb_max) =
                    Self::convert_terrain_vertices(vertices);
                ConvertedChunk {
                    positions,
                    normals,
                    tangents,
                    uvs,
                    biome_tint,
                    aabb_min,
                    aabb_max,
                    indices: indices.clone(),
                }
            })
            .collect();

        if converted.is_empty() {
            return;
        }

        // ── Spatial clustering (grid) ──────────────────────────────────
        // Instead of merging all 121 chunks into 1 draw call (no frustum
        // culling possible) we bin them into a grid of clusters. Each
        // cluster gets its own merged mesh + AABB → the renderer can cull
        // most terrain when the camera faces one direction.
        //
        // We use an adaptive grid: start with 8×8 so that 121 chunks stay
        // comfortably within wgpu's 256 MB buffer limit per draw call.
        let mut global_aabb_min = [f32::MAX; 3];
        let mut global_aabb_max = [f32::MIN; 3];
        for cc in &converted {
            for j in 0..3 {
                global_aabb_min[j] = global_aabb_min[j].min(cc.aabb_min[j]);
                global_aabb_max[j] = global_aabb_max[j].max(cc.aabb_max[j]);
            }
        }

        const GRID: usize = 4;
        // Maximum vertices per merged cluster.  48 bytes/vertex × 5M ≈ 240 MB,
        // safely under wgpu's 256 MB default buffer limit.
        const MAX_VERTICES_PER_BIN: usize = 5_000_000;
        let span_x = (global_aabb_max[0] - global_aabb_min[0]).max(1.0);
        let span_z = (global_aabb_max[2] - global_aabb_min[2]).max(1.0);
        let cell_w = span_x / GRID as f32;
        let cell_d = span_z / GRID as f32;

        // Bin chunks into grid cells by their AABB center
        let mut bins: Vec<Vec<usize>> = vec![Vec::new(); GRID * GRID];
        for (ci, cc) in converted.iter().enumerate() {
            let cx = ((cc.aabb_min[0] + cc.aabb_max[0]) * 0.5 - global_aabb_min[0]) / cell_w;
            let cz = ((cc.aabb_min[2] + cc.aabb_max[2]) * 0.5 - global_aabb_min[2]) / cell_d;
            let gx = (cx as usize).min(GRID - 1);
            let gz = (cz as usize).min(GRID - 1);
            bins[gz * GRID + gx].push(ci);
        }

        let mut dominant_tint = [0.9f32, 0.9, 0.9, 1.0];
        let mut sub_idx = 0u32; // global sub-cluster counter for unique names
        for (bin_idx, bin) in bins.iter().enumerate() {
            if bin.is_empty() {
                continue;
            }

            let mut merged_positions = Vec::new();
            let mut merged_normals = Vec::new();
            let mut merged_tangents = Vec::new();
            let mut merged_uvs = Vec::new();
            let mut merged_indices = Vec::new();
            let mut cluster_aabb_min = [f32::MAX; 3];
            let mut cluster_aabb_max = [f32::MIN; 3];
            let mut vertex_offset = 0u32;

            // Helper closure: flush the current merged buffers into a GPU model
            let flush = |positions: &mut Vec<[f32; 3]>,
                         normals: &mut Vec<[f32; 3]>,
                         tangents: &mut Vec<[f32; 4]>,
                         uvs: &mut Vec<[f32; 2]>,
                         indices: &mut Vec<u32>,
                         aabb_min: &mut [f32; 3],
                         aabb_max: &mut [f32; 3],
                         tint: [f32; 4],
                         renderer: &mut astraweave_render::Renderer,
                         names: &mut Vec<String>,
                         sub: &mut u32| {
                if positions.is_empty() {
                    return;
                }
                let name = format!("terrain_c{bin_idx}_{sub}");
                *sub += 1;
                let mesh = renderer.create_mesh_from_full_arrays(
                    positions, normals, tangents, uvs, indices,
                );
                let instance = astraweave_render::Instance::from_pos_scale_color(
                    glam::Vec3::ZERO,
                    glam::Vec3::ONE,
                    tint,
                );
                renderer.add_model_with_bounds(&name, mesh, &[instance], *aabb_min, *aabb_max);
                names.push(name);
                positions.clear();
                normals.clear();
                tangents.clear();
                uvs.clear();
                indices.clear();
                *aabb_min = [f32::MAX; 3];
                *aabb_max = [f32::MIN; 3];
            };

            for &ci in bin {
                let cc = &converted[ci];

                // If adding this chunk would exceed the per-buffer limit, flush first
                if !merged_positions.is_empty()
                    && merged_positions.len() + cc.positions.len() > MAX_VERTICES_PER_BIN
                {
                    vertex_offset = 0;
                    flush(
                        &mut merged_positions,
                        &mut merged_normals,
                        &mut merged_tangents,
                        &mut merged_uvs,
                        &mut merged_indices,
                        &mut cluster_aabb_min,
                        &mut cluster_aabb_max,
                        dominant_tint,
                        &mut self.renderer,
                        &mut self.terrain_model_names,
                        &mut sub_idx,
                    );
                }

                for &idx in &cc.indices {
                    merged_indices.push(idx + vertex_offset);
                }
                vertex_offset += cc.positions.len() as u32;

                merged_positions.extend_from_slice(&cc.positions);
                merged_normals.extend_from_slice(&cc.normals);
                merged_tangents.extend_from_slice(&cc.tangents);
                merged_uvs.extend_from_slice(&cc.uvs);

                for j in 0..3 {
                    cluster_aabb_min[j] = cluster_aabb_min[j].min(cc.aabb_min[j]);
                    cluster_aabb_max[j] = cluster_aabb_max[j].max(cc.aabb_max[j]);
                }
                dominant_tint = cc.biome_tint;
            }

            // Flush remaining
            flush(
                &mut merged_positions,
                &mut merged_normals,
                &mut merged_tangents,
                &mut merged_uvs,
                &mut merged_indices,
                &mut cluster_aabb_min,
                &mut cluster_aabb_max,
                dominant_tint,
                &mut self.renderer,
                &mut self.terrain_model_names,
                &mut sub_idx,
            );
        }

        // Track stats for the scene stats panel
        self.terrain_total_indices = total_indices;
        self.terrain_total_triangles = total_indices / 3;

        // Position ground fill plane below all terrain to block sky bleed-through.
        let global_min_y = global_aabb_min[1];
        let global_max_extent = global_aabb_max[0]
            .abs()
            .max(global_aabb_max[2].abs())
            .max(global_aabb_min[0].abs())
            .max(global_aabb_min[2].abs());

        if global_min_y < f32::MAX {
            let ground_y = global_min_y - 5.0;
            let extent = global_max_extent + 100.0;
            self.renderer.set_terrain_ground_plane(ground_y, extent);

            // ── Fog ────────────────────────────────────────────────────
            // Distance fog fades terrain edges smoothly into the sky.
            // The fog color MUST match the sky horizon color so the
            // transition is seamless — a mismatch creates white void.
            let env = self.renderer.scene_environment_mut();
            env.visuals.fog_color = glam::Vec3::new(0.75, 0.85, 1.0); // matches day_color_horizon
            env.visuals.fog_start = extent * 0.7; // begin further out
            env.visuals.fog_end = extent * 2.5; // softer rolloff
            env.visuals.fog_density = 0.06 / extent; // gentler exponential
                                                     // Ambient fill so shadowed areas aren't pitch black
            env.visuals.ambient_color = glam::Vec3::new(0.45, 0.50, 0.55);
            env.visuals.ambient_intensity = 0.35;
            tracing::debug!(
                "Ground fill plane set at Y={ground_y:.1}, extent={extent:.0}, \
                 fog_start={:.0}, fog_density={:.5}",
                env.visuals.fog_start,
                env.visuals.fog_density,
            );

            // ── Sky ────────────────────────────────────────────────────
            // Activate the procedural sky renderer so the background shows
            // a gradient sky instead of a flat white/grey void.
            let sky = astraweave_render::SkyConfig {
                day_color_top: glam::Vec3::new(0.25, 0.55, 1.0),
                day_color_horizon: glam::Vec3::new(0.75, 0.85, 1.0),
                sunset_color_top: glam::Vec3::new(0.8, 0.4, 0.2),
                sunset_color_horizon: glam::Vec3::new(1.0, 0.6, 0.3),
                night_color_top: glam::Vec3::new(0.0, 0.0, 0.1),
                night_color_horizon: glam::Vec3::new(0.1, 0.1, 0.2),
                cloud_coverage: 0.35,
                cloud_speed: 0.01,
                cloud_altitude: 800.0,
            };
            self.renderer.set_sky_config(sky);

            // ── Sun ────────────────────────────────────────────────────
            // A warm directional light at ~35° elevation for visible
            // terrain shadows and natural surface shading.
            let sun_dir = glam::Vec3::new(-0.5, -0.6, -0.4).normalize();
            self.renderer.set_light_direction_override(sun_dir, 1.5);

            // ── Shadow cascade tuning for terrain ──────────────────────
            // Wider cascade extents cover more terrain, and a higher
            // cascade lambda biases toward logarithmic splits for better
            // near-field shadow quality.
            self.renderer.set_cascade_extents(80.0, 250.0);
            self.renderer.set_cascade_lambda(0.7);
            self.renderer.set_shadow_filter(1.5, 0.0005, 1.0);

            // ── Quality preset ─────────────────────────────────────────
            // Auto-switch to EditorTerrain (shadows) when terrain
            // is loaded, unless the user has explicitly chosen GameQuality.
            if self.quality_preset != EditorQualityPreset::GameQuality {
                self.apply_quality_preset(EditorQualityPreset::EditorTerrain);
            }
        }

        tracing::debug!(
            "Uploaded {} terrain chunks ({} triangles) to engine renderer",
            self.terrain_model_names.len(),
            self.terrain_total_triangles,
        );
    }

    /// Convert editor terrain vertices to engine mesh arrays in a single pass.
    ///
    /// Avoids the intermediate `Vec<astraweave_render::TerrainVertex>` allocation
    /// and separates position/normal/tangent/UV in one iteration. Also computes
    /// the dominant biome tint and world-space AABB for frustum culling.
    fn convert_terrain_vertices(
        vertices: &[TerrainVertex],
    ) -> (
        Vec<[f32; 3]>,
        Vec<[f32; 3]>,
        Vec<[f32; 4]>,
        Vec<[f32; 2]>,
        [f32; 4],
        [f32; 3],
        [f32; 3],
    ) {
        let count = vertices.len();
        let mut positions = Vec::with_capacity(count);
        let mut normals = Vec::with_capacity(count);
        let mut tangents = Vec::with_capacity(count);
        let mut uvs = Vec::with_capacity(count);
        let mut biome_counts = [0u32; 8];
        let mut aabb_min = [f32::MAX; 3];
        let mut aabb_max = [f32::MIN; 3];

        for v in vertices {
            positions.push(v.position);
            normals.push(v.normal);
            uvs.push(v.uv);

            // Compute tangent from normal
            let n = glam::Vec3::from(v.normal);
            let up = if n.y.abs() > 0.99 {
                glam::Vec3::X
            } else {
                glam::Vec3::Y
            };
            let t = n.cross(up).normalize();
            tangents.push([t.x, t.y, t.z, 1.0]);

            // Track dominant biome
            let weights = [
                v.biome_weights_0[0],
                v.biome_weights_0[1],
                v.biome_weights_0[2],
                v.biome_weights_0[3],
                v.biome_weights_1[0],
                v.biome_weights_1[1],
                v.biome_weights_1[2],
                v.biome_weights_1[3],
            ];
            let mut best_idx = 0;
            let mut best_w = weights[0];
            for (i, &w) in weights.iter().enumerate().skip(1) {
                if w > best_w {
                    best_w = w;
                    best_idx = i;
                }
            }
            biome_counts[best_idx.min(7)] += 1;

            // AABB
            for j in 0..3 {
                aabb_min[j] = aabb_min[j].min(v.position[j]);
                aabb_max[j] = aabb_max[j].max(v.position[j]);
            }
        }

        // Dominant biome tint
        let dominant = biome_counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let tint = Self::biome_id_to_tint(dominant as u32);

        (positions, normals, tangents, uvs, tint, aabb_min, aabb_max)
    }

    /// Map a biome ID (0-7) to an instance tint color.
    ///
    /// These multiply with the albedo texture (PBR shader: `base_color *= input.color`),
    /// so values should be near 1.0 with a slight color bias — not dark colors.
    fn biome_id_to_tint(biome_id: u32) -> [f32; 4] {
        match biome_id {
            0 => [0.80, 1.00, 0.70, 1.0], // Grassland — warm green boost
            1 => [1.10, 1.00, 0.75, 1.0], // Desert — warm sandy shift
            2 => [0.60, 0.85, 0.50, 1.0], // Forest — deeper green
            3 => [0.85, 0.85, 0.80, 1.0], // Mountain — cool gray
            4 => [1.05, 1.05, 1.10, 1.0], // Tundra — cool bright
            5 => [0.70, 0.80, 0.55, 1.0], // Swamp — olive
            6 => [1.10, 1.05, 0.85, 1.0], // Beach — warm sand
            7 => [0.75, 0.90, 0.85, 1.0], // River — cool blue-green
            _ => [0.90, 0.90, 0.90, 1.0], // Unknown — neutral
        }
    }

    /// Clear all terrain data from the engine renderer.
    pub fn clear_terrain(&mut self) {
        for name in self.terrain_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        self.terrain_total_triangles = 0;
        self.terrain_total_indices = 0;
        self.terrain_cached_indices.clear();
        // Restore default ground plane position
        self.renderer.reset_ground_plane();
    }

    /// Get the number of terrain chunks currently loaded in the engine.
    pub fn terrain_chunk_count(&self) -> usize {
        self.terrain_model_names.len()
    }

    /// Total terrain triangles across all uploaded chunks.
    pub fn terrain_triangles(&self) -> usize {
        self.terrain_total_triangles
    }

    /// Total terrain indices across all uploaded chunks.
    pub fn terrain_indices(&self) -> usize {
        self.terrain_total_indices
    }

    /// Incrementally update a single terrain chunk's vertices on the GPU.
    ///
    /// Replaces the model named `terrain_chunk_{chunk_index}` with a fresh
    /// mesh built from the provided vertices. Uses cached indices from the
    /// initial upload (brush strokes change heights/normals, not topology).
    pub fn update_terrain_chunk(&mut self, chunk_index: usize, vertices: &[TerrainVertex]) {
        let name = format!("terrain_chunk_{chunk_index}");
        if vertices.is_empty() {
            return;
        }

        let indices = match self.terrain_cached_indices.get(chunk_index) {
            Some(idx) => idx,
            None => {
                tracing::warn!("update_terrain_chunk: no cached indices for chunk {chunk_index}");
                return;
            }
        };

        // Single-pass conversion (avoids intermediate Vec<TerrainVertex> allocation)
        let (positions, normals, tangents, uvs, biome_tint, _, _) =
            Self::convert_terrain_vertices(vertices);

        let mesh = self
            .renderer
            .create_mesh_from_full_arrays(&positions, &normals, &tangents, &uvs, indices);
        let instance = astraweave_render::Instance::from_pos_scale_color(
            glam::Vec3::ZERO,
            glam::Vec3::ONE,
            biome_tint,
        );

        // add_model with the same name replaces the existing entry in the HashMap
        self.renderer.add_model(&name, mesh, &[instance]);
    }

    // ── Scatter / vegetation feeding ────────────────────────────────────

    /// Upload scatter placements as instanced models in the engine renderer.
    ///
    /// Groups placements by mesh key, creates one model per unique mesh type.
    /// Upload scatter placements. Returns (loaded, total_groups, not_found, load_failed).
    pub fn upload_scatter_placements(
        &mut self,
        placements: &[ScatterPlacement],
        diffuse_textures: &std::collections::HashMap<String, std::path::PathBuf>,
    ) -> (u32, u32, u32, u32) {
        // Clear previous scatter models
        for name in self.scatter_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }

        if placements.is_empty() {
            return (0, 0, 0, 0);
        }

        // Group by mesh_key
        let mut groups: std::collections::HashMap<String, Vec<&ScatterPlacement>> =
            std::collections::HashMap::new();
        for p in placements {
            groups.entry(p.mesh_key.clone()).or_default().push(p);
        }

        let mut loaded_groups = 0u32;
        let mut skipped_not_found = 0u32;
        let mut skipped_load_fail = 0u32;

        // Texture deduplication cache: canonical path → (width, height, rgba_pixels).
        // Prevents the same image file from being loaded and decoded multiple times
        // when several scatter groups reference the same texture.
        let mut texture_cache: std::collections::HashMap<std::path::PathBuf, (u32, u32, Vec<u8>)> =
            std::collections::HashMap::new();

        for (key, items) in &groups {
            // Only render scatter groups that have real glTF mesh assets on disk.
            let mesh_path = &items[0].mesh_path;
            let mesh_path_obj = std::path::Path::new(mesh_path);
            if !mesh_path_obj.exists() {
                // On the first few misses, log the full path for debugging
                if skipped_not_found < 3 {
                    tracing::warn!(
                        target: "aw_editor::viewport",
                        "Scatter: skipping '{}' ({} instances) — mesh not found: {}",
                        key,
                        items.len(),
                        mesh_path
                    );
                }
                skipped_not_found += 1;
                continue;
            }

            // Load glTF mesh and add as instanced model with all placements.
            // Wrapped in catch_unwind to prevent a single bad asset from
            // crashing the entire editor (some .gltf files reference missing
            // textures or have incompatible formats).
            let path = std::path::Path::new(mesh_path);
            let load_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let opts = astraweave_render::mesh_gltf::GltfOptions::default();
                astraweave_render::mesh_gltf::load_gltf(path, &opts)
            }));
            match load_result {
                Ok(Ok(cpu_meshes)) if !cpu_meshes.is_empty() => {
                    // Compute model AABB for pivot correction: most glTF models
                    // have their origin at the center of the bounding box rather
                    // than at the bottom. `aabb_min_y * scale` gives the offset
                    // needed to place the model's base at ground level.
                    let aabb_min_y = cpu_meshes[0].aabb().map(|(min, _max)| min.y).unwrap_or(0.0);

                    // Build per-instance data with AABB pivot correction and
                    // surface-normal alignment, paired with the source placement
                    // for spatial binning.
                    let instance_data: Vec<(glam::Vec3, astraweave_render::Instance)> = items
                        .iter()
                        .map(|p| {
                            let normal_quat = {
                                let n = p.terrain_normal;
                                let slope_cos = n.y;
                                if slope_cos < 0.996 && n.length_squared() > 0.5 {
                                    glam::Quat::from_rotation_arc(glam::Vec3::Y, n)
                                } else {
                                    glam::Quat::IDENTITY
                                }
                            };
                            let yaw_quat = glam::Quat::from_rotation_y(p.rotation);
                            let rotation = normal_quat * yaw_quat;

                            let pivot_offset = aabb_min_y * p.scale;
                            let mut pos = p.position;
                            pos.y -= pivot_offset;

                            let transform = glam::Mat4::from_scale_rotation_translation(
                                glam::Vec3::splat(p.scale),
                                rotation,
                                pos,
                            );
                            (
                                p.position,
                                astraweave_render::Instance {
                                    transform,
                                    color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                                    material_id: 0,
                                },
                            )
                        })
                        .collect();

                    // ── Spatial sub-grouping (2×2 quadrants) ───────────────
                    // Splitting into quadrants gives the frustum culler 4×
                    // granularity: when the camera faces one direction, the
                    // back quadrants are culled entirely.
                    let mut x_min = f32::MAX;
                    let mut x_max = f32::MIN;
                    let mut z_min = f32::MAX;
                    let mut z_max = f32::MIN;
                    for p in items.iter() {
                        x_min = x_min.min(p.position.x);
                        x_max = x_max.max(p.position.x);
                        z_min = z_min.min(p.position.z);
                        z_max = z_max.max(p.position.z);
                    }
                    let mid_x = (x_min + x_max) * 0.5;
                    let mid_z = (z_min + z_max) * 0.5;

                    // Bin instances into 4 quadrants
                    let mut quadrants: [Vec<(glam::Vec3, astraweave_render::Instance)>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for (pos, inst) in instance_data {
                        let qi = match (pos.x >= mid_x, pos.z >= mid_z) {
                            (false, false) => 0,
                            (true, false) => 1,
                            (false, true) => 2,
                            (true, true) => 3,
                        };
                        quadrants[qi].push((pos, inst));
                    }

                    let has_texture = cpu_meshes[0].albedo_image.is_some();

                    // If glTF has no embedded texture (decomposed assets), try
                    // loading the diffuse texture from the BiomePack texture map.
                    //
                    // Textures are capped at SCATTER_MAX_TEX_SIZE to prevent GPU
                    // memory exhaustion from oversized assets (e.g., 8192×8192
                    // normal maps mislabeled as diffuse).  Deduplication by
                    // canonical path avoids re-loading the same image file for
                    // multiple scatter groups.
                    const SCATTER_MAX_TEX_SIZE: u32 = 2048;

                    let external_texture = if !has_texture {
                        // Try matching by mesh filename stem
                        let stem = std::path::Path::new(mesh_path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");
                        let tex_path = diffuse_textures
                            .get(stem)
                            .or_else(|| diffuse_textures.get(key.as_str()));
                        tex_path.and_then(|p| {
                            if !p.exists() {
                                tracing::warn!(
                                    "Scatter: diffuse texture not found: {}",
                                    p.display()
                                );
                                return None;
                            }

                            // Check the texture cache first (deduplicate by
                            // canonical path so the same file isn't loaded and
                            // uploaded multiple times).
                            let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
                            if let Some(cached) = texture_cache.get(&canon) {
                                tracing::debug!(
                                    "Scatter: reusing cached {}×{} texture for '{key}'",
                                    cached.0,
                                    cached.1
                                );
                                return Some(cached.clone());
                            }

                            match image::open(p) {
                                Ok(img) => {
                                    let rgba = img.to_rgba8();
                                    let (w, h) = rgba.dimensions();

                                    // Cap at SCATTER_MAX_TEX_SIZE to prevent GPU OOM
                                    let (final_w, final_h, pixels) =
                                        if w > SCATTER_MAX_TEX_SIZE || h > SCATTER_MAX_TEX_SIZE {
                                            let tw = w.min(SCATTER_MAX_TEX_SIZE);
                                            let th = h.min(SCATTER_MAX_TEX_SIZE);
                                            tracing::info!(
                                                target: "aw_editor::viewport",
                                                "Scatter: downsampling {}×{} → {}×{} for '{key}' from {}",
                                                w, h, tw, th, p.display()
                                            );
                                            let resized = image::imageops::resize(
                                                &rgba,
                                                tw,
                                                th,
                                                image::imageops::FilterType::Triangle,
                                            );
                                            (tw, th, resized.into_raw())
                                        } else {
                                            tracing::debug!(
                                                "Scatter: loaded diffuse {}×{} for '{key}' from {}",
                                                w, h, p.display()
                                            );
                                            (w, h, rgba.into_raw())
                                        };

                                    let entry = (final_w, final_h, pixels);
                                    texture_cache.insert(canon, entry.clone());
                                    Some(entry)
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Scatter: failed to load diffuse for '{key}': {e}"
                                    );
                                    None
                                }
                            }
                        })
                    } else {
                        None
                    };

                    // ── Create mesh ONCE per group and track the first
                    // textured quadrant so subsequent quadrants share the
                    // GPU texture (wgpu resources are reference-counted). ──
                    let shared_mesh = self.renderer.create_mesh_from_cpu_mesh(&cpu_meshes[0]);
                    let mut first_textured_quad: Option<String> = None;

                    for (qi, quad) in quadrants.iter().enumerate() {
                        if quad.is_empty() {
                            continue;
                        }
                        let sub_name = format!("scatter_{key}_q{qi}");
                        let instances: Vec<astraweave_render::Instance> =
                            quad.iter().map(|(_, inst)| inst.clone()).collect();

                        let mesh = shared_mesh.clone();

                        // If a prior quadrant already uploaded the texture,
                        // share it instead of creating a duplicate GPU texture.
                        if let Some(ref source) = first_textured_quad {
                            if self
                                .renderer
                                .add_model_sharing_texture(&sub_name, mesh.clone(), &instances, source)
                            {
                                // Texture shared successfully — skip texture upload.
                            } else {
                                // Source disappeared (shouldn't happen) — fall back.
                                self.renderer.add_model(&sub_name, mesh, &instances);
                            }
                        } else if let Some(img) = cpu_meshes[0].albedo_image.as_ref() {
                            // Cap embedded textures to prevent GPU OOM
                            if img.width > SCATTER_MAX_TEX_SIZE || img.height > SCATTER_MAX_TEX_SIZE
                            {
                                let src = image::RgbaImage::from_raw(
                                    img.width,
                                    img.height,
                                    img.pixels.clone(),
                                );
                                if let Some(src_img) = src {
                                    let tw = img.width.min(SCATTER_MAX_TEX_SIZE);
                                    let th = img.height.min(SCATTER_MAX_TEX_SIZE);
                                    let resized = image::imageops::resize(
                                        &src_img,
                                        tw,
                                        th,
                                        image::imageops::FilterType::Triangle,
                                    );
                                    self.renderer.add_model_with_texture(
                                        &sub_name,
                                        mesh,
                                        &instances,
                                        tw,
                                        th,
                                        &resized.into_raw(),
                                    );
                                    first_textured_quad = Some(sub_name.clone());
                                } else {
                                    tracing::warn!(
                                        target: "aw_editor::viewport",
                                        "Scatter: embedded texture {}×{} could not be resized; rendering untextured",
                                        img.width, img.height
                                    );
                                    self.renderer.add_model(&sub_name, mesh, &instances);
                                }
                            } else {
                                self.renderer.add_model_with_texture(
                                    &sub_name,
                                    mesh,
                                    &instances,
                                    img.width,
                                    img.height,
                                    &img.pixels,
                                );
                                first_textured_quad = Some(sub_name.clone());
                            }
                        } else if let Some((w, h, ref pixels)) = external_texture {
                            self.renderer
                                .add_model_with_texture(&sub_name, mesh, &instances, w, h, pixels);
                            first_textured_quad = Some(sub_name.clone());
                        } else {
                            self.renderer.add_model(&sub_name, mesh, &instances);
                        }

                        // Compute tight AABB for this quadrant
                        let mut g_min = [f32::MAX; 3];
                        let mut g_max = [f32::MIN; 3];
                        let br = items[0].bounding_radius;
                        for (pos, _) in quad.iter() {
                            g_min[0] = g_min[0].min(pos.x - br);
                            g_min[1] = g_min[1].min(pos.y - br);
                            g_min[2] = g_min[2].min(pos.z - br);
                            g_max[0] = g_max[0].max(pos.x + br);
                            g_max[1] = g_max[1].max(pos.y + br);
                            g_max[2] = g_max[2].max(pos.z + br);
                        }
                        self.renderer.set_model_bounds(&sub_name, g_min, g_max);

                        self.scatter_model_names.push(sub_name);
                    }

                    loaded_groups += 1;
                }
                Ok(Ok(_)) => {
                    tracing::warn!("Scatter: '{key}' glTF has no meshes: {mesh_path}");
                    skipped_load_fail += 1;
                }
                Ok(Err(e)) => {
                    tracing::warn!("Scatter: skipping '{key}' — glTF load failed: {e}");
                    skipped_load_fail += 1;
                }
                Err(_) => {
                    tracing::warn!("Scatter: '{key}' glTF load panicked — skipping: {mesh_path}");
                    skipped_load_fail += 1;
                }
            }
        }

        self.scatter_placement_count = placements.len();
        self.scatter_draw_call_count = loaded_groups;

        let total = groups.len() as u32;
        tracing::info!(
            target: "aw_editor::viewport",
            "Scatter upload: {loaded_groups}/{total} mesh groups loaded, {skipped_not_found} not found, {skipped_load_fail} load failed, {} instances, {} cached textures",
            placements.len(),
            texture_cache.len(),
        );

        // Return summary for console display
        (loaded_groups, total, skipped_not_found, skipped_load_fail)
    }

    /// Clear all scatter data from the engine renderer.
    pub fn clear_scatter(&mut self) {
        for name in self.scatter_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        self.scatter_placement_count = 0;
        self.scatter_draw_call_count = 0;
    }

    /// Total scatter placements currently loaded.
    pub fn scatter_instance_count(&self) -> usize {
        self.scatter_placement_count
    }

    /// Number of unique scatter draw calls (one per mesh type).
    pub fn scatter_draw_calls(&self) -> u32 {
        self.scatter_draw_call_count
    }

    // ── Sky / weather / environment ─────────────────────────────────────

    /// Set the sky configuration on the engine renderer.
    pub fn set_sky_config(&mut self, cfg: astraweave_render::SkyConfig) {
        self.renderer.set_sky_config(cfg);
    }

    /// Load an HDRI file as the skybox and rebake IBL environment maps.
    pub fn load_hdri(&mut self, path: &std::path::Path) -> Result<()> {
        let path_str = path.to_string_lossy().to_string();
        tracing::info!("Loading HDRI skybox from: {path_str}");
        self.renderer.ibl_mut().mode = astraweave_render::ibl::SkyMode::HdrPath {
            biome: "editor".to_string(),
            path: path_str,
        };
        self.renderer
            .bake_environment(astraweave_render::ibl::IblQuality::Medium)
            .context("Failed to bake HDRI environment")?;
        tracing::info!("HDRI skybox loaded and IBL baked successfully");
        Ok(())
    }

    /// Get the current sky configuration.
    pub fn sky_config(&self) -> astraweave_render::SkyConfig {
        self.renderer.sky_config()
    }

    /// Set weather type on the engine renderer.
    pub fn set_weather(&mut self, kind: astraweave_render::WeatherKind) {
        self.weather_active = kind != astraweave_render::WeatherKind::None;
        self.renderer.set_weather(kind);
    }

    /// Whether weather effects are currently active.
    pub fn weather_active(&self) -> bool {
        self.weather_active
    }

    /// Tick the weather particle system.
    pub fn tick_weather(&mut self, dt: f32) {
        self.renderer.tick_weather(dt);
    }

    /// Advance the environment (time-of-day, sky parameters).
    pub fn tick_environment(&mut self, dt: f32) {
        self.renderer.tick_environment(dt);
    }

    // ── Fog / lighting ──────────────────────────────────────────────────

    /// Apply fog parameters to the engine's scene environment.
    ///
    /// Only overwrites fog when the world panel's fog toggle is enabled.
    /// When disabled, terrain-scaled fog values (set by `upload_terrain_chunks`)
    /// are preserved.
    pub fn set_fog_params(&mut self, params: &TerrainFogParams) {
        if params.fog_enabled {
            let env = self.renderer.scene_environment_mut();
            env.visuals.fog_density = params.fog_density;
            env.visuals.fog_color = glam::Vec3::from(params.fog_color);
        }
        // Apply particle count override from the UI slider
        if let Some(count) = params.particle_count_override {
            self.renderer.set_weather_max(count as usize);
        }
    }

    /// Apply lighting parameters to the engine's scene environment and camera UBO.
    ///
    /// Updates ambient lighting in the SceneEnvironment, and overrides the
    /// sun direction + intensity in the CameraUBO so the PBR shader uses
    /// the world panel's lighting settings instead of the internal TimeOfDay.
    pub fn set_lighting_params(&mut self, params: &TerrainLightingParams) {
        let env = self.renderer.scene_environment_mut();
        env.visuals.ambient_color = glam::Vec3::from(params.ambient_color);
        env.visuals.ambient_intensity = params.ambient_intensity;
        env.sun_color = params.sun_color;
        env.sun_intensity = params.sun_intensity;

        // Negate: sun_dir points TO the sun (positive Y = sun above),
        // but the shader convention for light_dir is direction FROM the
        // sun (negative Y = light traveling downward). The shader then
        // does L = normalize(-light_dir) to get the direction toward
        // the light source.
        let dir = (-glam::Vec3::from(params.sun_dir)).normalize();
        self.renderer
            .set_light_direction_override(dir, params.sun_intensity);
    }

    /// Set water configuration on the engine renderer.
    pub fn set_water_enabled(&mut self, enabled: bool, style: WaterStyle) {
        self.water_enabled = enabled;
        if enabled {
            let format = self.renderer.surface_format();
            let water = astraweave_render::WaterRenderer::new(
                self.renderer.device(),
                format,
                wgpu::TextureFormat::Depth32Float,
            );
            // Apply style-specific colors
            let (deep, shallow, foam) = match style {
                WaterStyle::Ocean => (
                    glam::Vec3::new(0.02, 0.08, 0.2),
                    glam::Vec3::new(0.1, 0.4, 0.5),
                    glam::Vec3::new(0.95, 0.98, 1.0),
                ),
                WaterStyle::River => (
                    glam::Vec3::new(0.01, 0.05, 0.04),
                    glam::Vec3::new(0.04, 0.10, 0.08),
                    glam::Vec3::new(0.9, 0.95, 0.9),
                ),
                WaterStyle::Lake => (
                    glam::Vec3::new(0.005, 0.04, 0.06),
                    glam::Vec3::new(0.02, 0.09, 0.12),
                    glam::Vec3::new(0.9, 0.95, 1.0),
                ),
                WaterStyle::Swamp => (
                    glam::Vec3::new(0.02, 0.03, 0.01),
                    glam::Vec3::new(0.05, 0.06, 0.03),
                    glam::Vec3::new(0.7, 0.75, 0.6),
                ),
            };
            let mut water = water;
            water.set_water_colors(deep, shallow, foam);
            self.renderer.set_water_renderer(water);
        } else {
            self.renderer.clear_water_renderer();
        }
    }

    /// Update water animation state each frame.
    pub fn update_water(&mut self, camera: &OrbitCamera, time: f32) {
        let engine_camera = camera.to_engine_camera();
        let vp = engine_camera.vp();
        let pos = camera.position();
        self.renderer.update_water(vp, pos, time);
    }
}

// ─── Default cube geometry for entities without meshes ──────────────────────
#[rustfmt::skip]
const CUBE_POSITIONS: [[f32; 3]; 24] = [
    // Front face (+Z)
    [-0.5, -0.5,  0.5], [ 0.5, -0.5,  0.5], [ 0.5,  0.5,  0.5], [-0.5,  0.5,  0.5],
    // Back face (-Z)
    [ 0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5,  0.5, -0.5], [ 0.5,  0.5, -0.5],
    // Top face (+Y)
    [-0.5,  0.5,  0.5], [ 0.5,  0.5,  0.5], [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5],
    // Bottom face (-Y)
    [-0.5, -0.5, -0.5], [ 0.5, -0.5, -0.5], [ 0.5, -0.5,  0.5], [-0.5, -0.5,  0.5],
    // Right face (+X)
    [ 0.5, -0.5,  0.5], [ 0.5, -0.5, -0.5], [ 0.5,  0.5, -0.5], [ 0.5,  0.5,  0.5],
    // Left face (-X)
    [-0.5, -0.5, -0.5], [-0.5, -0.5,  0.5], [-0.5,  0.5,  0.5], [-0.5,  0.5, -0.5],
];

#[rustfmt::skip]
const CUBE_NORMALS: [[f32; 3]; 24] = [
    // Front
    [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0],
    // Back
    [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0, -1.0],
    // Top
    [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0],
    // Bottom
    [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, -1.0, 0.0],
    // Right
    [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 0.0],
    // Left
    [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0],
];

#[rustfmt::skip]
const CUBE_INDICES: [u32; 36] = [
    0,  1,  2,  2,  3,  0,   // Front
    4,  5,  6,  6,  7,  4,   // Back
    8,  9,  10, 10, 11, 8,   // Top
    12, 13, 14, 14, 15, 12,  // Bottom
    16, 17, 18, 18, 19, 16,  // Right
    20, 21, 22, 22, 23, 20,  // Left
];
