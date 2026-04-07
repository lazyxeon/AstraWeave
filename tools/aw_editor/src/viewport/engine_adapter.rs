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
            present_mode: wgpu::PresentMode::Fifo,
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

        Ok(Self {
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
        })
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

        // Clear previous entity models (prefixed with "entity_")
        let old_names: Vec<String> = self
            .renderer
            .model_names()
            .into_iter()
            .filter(|n| n.starts_with("entity_"))
            .collect();
        for name in &old_names {
            self.renderer.clear_model(name);
        }

        // Add each group as a named model
        for (mesh_key, instances) in &mesh_groups {
            let model_name = match mesh_key {
                Some(path) => format!("entity_mesh_{}", path.replace(['/', '\\', '.'], "_")),
                None => "entity_default_cubes".to_string(),
            };

            // Load mesh if not already in engine (lazy load)
            if !self.renderer.has_model(&model_name) {
                let mesh = match mesh_key {
                    Some(path) => {
                        // Try to load glTF mesh
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
            } else {
                // Model mesh already loaded — just update instances
                self.renderer.clear_model(&model_name);
                let mesh = match mesh_key {
                    Some(path) => {
                        let opts = astraweave_render::mesh_gltf::GltfOptions::default();
                        match astraweave_render::mesh_gltf::load_gltf(Path::new(path), &opts) {
                            Ok(cpu_meshes) if !cpu_meshes.is_empty() => {
                                self.renderer.create_mesh_from_cpu_mesh(&cpu_meshes[0])
                            }
                            _ => self.renderer.create_mesh_from_arrays(
                                &CUBE_POSITIONS,
                                &CUBE_NORMALS,
                                &CUBE_INDICES,
                            ),
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
        }
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

        for (i, (vertices, indices)) in chunks.iter().enumerate() {
            if vertices.is_empty() || indices.is_empty() {
                continue;
            }
            let name = format!("terrain_chunk_{i}");

            // Convert editor TerrainVertex (96 bytes) → engine format.
            // to_engine_vertex() extracts dominant biome ID from the 8 weight slots,
            // preserving biome information for the engine's terrain shader.
            let engine_verts: Vec<astraweave_render::TerrainVertex> =
                vertices.iter().map(|v| v.to_engine_vertex()).collect();

            // Determine dominant biome for this chunk to tint the instance color.
            // The PBR shader multiplies input.color.rgb into base_color (pbr.wgsl:125),
            // so the biome tint modulates the albedo texture per-chunk.
            let biome_tint = Self::dominant_biome_tint(&engine_verts);

            let positions: Vec<[f32; 3]> = engine_verts.iter().map(|v| v.position).collect();
            let normals: Vec<[f32; 3]> = engine_verts.iter().map(|v| v.normal).collect();
            let uvs: Vec<[f32; 2]> = engine_verts.iter().map(|v| v.uv).collect();
            // Generate tangents from normal (approximate — terrain doesn't need
            // perfect tangent frames for engine-level rendering).
            let tangents: Vec<[f32; 4]> = normals
                .iter()
                .map(|n| {
                    let up = if n[1].abs() > 0.99 {
                        glam::Vec3::X
                    } else {
                        glam::Vec3::Y
                    };
                    let t = glam::Vec3::from(*n).cross(up).normalize();
                    [t.x, t.y, t.z, 1.0]
                })
                .collect();

            // Compute world-space AABB for frustum culling
            let (mut aabb_min, mut aabb_max) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &positions {
                for j in 0..3 {
                    aabb_min[j] = aabb_min[j].min(p[j]);
                    aabb_max[j] = aabb_max[j].max(p[j]);
                }
            }

            let mesh = self
                .renderer
                .create_mesh_from_full_arrays(&positions, &normals, &tangents, &uvs, indices);

            let instance = astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::ZERO,
                glam::Vec3::ONE,
                biome_tint,
            );

            self.renderer
                .add_model_with_bounds(&name, mesh, &[instance], aabb_min, aabb_max);
            self.terrain_model_names.push(name);
        }

        // Track stats for the scene stats panel
        self.terrain_total_indices = chunks.iter().map(|(_, idx)| idx.len()).sum();
        self.terrain_total_triangles = self.terrain_total_indices / 3;

        // Position ground fill plane below all terrain to block sky bleed-through.
        // Compute global min Y and max XZ extent across all chunks.
        let mut global_min_y = f32::MAX;
        let mut global_max_extent: f32 = 50.0;
        for (verts, _) in chunks {
            for v in verts {
                global_min_y = global_min_y.min(v.position[1]);
                global_max_extent = global_max_extent
                    .max(v.position[0].abs())
                    .max(v.position[2].abs());
            }
        }
        if global_min_y < f32::MAX {
            let ground_y = global_min_y - 5.0; // 5 units below lowest terrain point
            let extent = global_max_extent + 100.0; // generous overshoot
            self.renderer.set_terrain_ground_plane(ground_y, extent);

            // Scale fog parameters for the terrain extent so distant chunks
            // fade gently rather than disappearing into a wall of white.
            let env = self.renderer.scene_environment_mut();
            env.visuals.fog_start = extent * 0.4; // fog begins at 40% of terrain extent
            env.visuals.fog_density = 0.5 / extent; // ~50% fog at the far edge
            tracing::debug!(
                "Ground fill plane set at Y={ground_y:.1}, extent={extent:.0}, \
                 fog_start={:.0}, fog_density={:.5}",
                env.visuals.fog_start,
                env.visuals.fog_density,
            );
        }

        tracing::debug!(
            "Uploaded {} terrain chunks ({} triangles) to engine renderer",
            self.terrain_model_names.len(),
            self.terrain_total_triangles,
        );
    }

    /// Compute a biome-appropriate tint color from the dominant biome in a chunk's vertices.
    ///
    /// Counts biome_id occurrences across all vertices and maps the most-frequent
    /// biome to an RGBA tint. The PBR shader multiplies `input.color.rgb` into
    /// `base_color`, so this tint modulates the albedo texture per-chunk.
    fn dominant_biome_tint(verts: &[astraweave_render::TerrainVertex]) -> [f32; 4] {
        let mut counts = [0u32; 8];
        for v in verts {
            let idx = (v.biome_id as usize).min(7);
            counts[idx] += 1;
        }
        let dominant = counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(0);
        Self::biome_id_to_tint(dominant as u32)
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

        let engine_verts: Vec<astraweave_render::TerrainVertex> =
            vertices.iter().map(|v| v.to_engine_vertex()).collect();
        let biome_tint = Self::dominant_biome_tint(&engine_verts);
        let positions: Vec<[f32; 3]> = engine_verts.iter().map(|v| v.position).collect();
        let normals: Vec<[f32; 3]> = engine_verts.iter().map(|v| v.normal).collect();
        let uvs: Vec<[f32; 2]> = engine_verts.iter().map(|v| v.uv).collect();
        let tangents: Vec<[f32; 4]> = normals
            .iter()
            .map(|n| {
                let up = if n[1].abs() > 0.99 {
                    glam::Vec3::X
                } else {
                    glam::Vec3::Y
                };
                let t = glam::Vec3::from(*n).cross(up).normalize();
                [t.x, t.y, t.z, 1.0]
            })
            .collect();

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
    pub fn upload_scatter_placements(&mut self, placements: &[ScatterPlacement]) {
        // Clear previous scatter models
        for name in self.scatter_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }

        if placements.is_empty() {
            return;
        }

        // Group by mesh_key
        let mut groups: std::collections::HashMap<String, Vec<&ScatterPlacement>> =
            std::collections::HashMap::new();
        for p in placements {
            groups.entry(p.mesh_key.clone()).or_default().push(p);
        }

        let mut loaded_groups = 0u32;
        for (key, items) in &groups {
            // Only render scatter groups that have real glTF mesh assets on disk.
            let mesh_path = &items[0].mesh_path;
            if !std::path::Path::new(mesh_path).exists() {
                tracing::debug!("Scatter: skipping '{key}' — mesh not found: {mesh_path}");
                continue;
            }

            let name = format!("scatter_{key}");

            // Build instances from placements
            let instances: Vec<astraweave_render::Instance> = items
                .iter()
                .map(|p| {
                    let transform = glam::Mat4::from_scale_rotation_translation(
                        glam::Vec3::splat(p.scale),
                        glam::Quat::from_rotation_y(p.rotation),
                        p.position,
                    );
                    astraweave_render::Instance {
                        transform,
                        color: [p.tint[0], p.tint[1], p.tint[2], 1.0],
                        material_id: 0,
                    }
                })
                .collect();

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
                    let mesh = self.renderer.create_mesh_from_cpu_mesh(&cpu_meshes[0]);
                    self.renderer.add_model(&name, mesh, &instances);
                    self.scatter_model_names.push(name);
                    loaded_groups += 1;
                }
                Ok(Ok(_)) => {
                    tracing::debug!("Scatter: '{key}' glTF has no meshes: {mesh_path}");
                }
                Ok(Err(e)) => {
                    tracing::debug!("Scatter: skipping '{key}' — glTF load failed: {e}");
                }
                Err(_) => {
                    tracing::warn!("Scatter: '{key}' glTF load panicked — skipping: {mesh_path}");
                }
            }
        }

        self.scatter_placement_count = placements.len();
        self.scatter_draw_call_count = loaded_groups;

        tracing::debug!(
            "Scatter: {loaded_groups}/{} groups loaded ({} placements, {} skipped without mesh)",
            groups.len(),
            placements.len(),
            groups.len() as u32 - loaded_groups,
        );
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
    pub fn set_fog_params(&mut self, params: &TerrainFogParams) {
        let env = self.renderer.scene_environment_mut();
        if params.fog_enabled {
            env.visuals.fog_density = params.fog_density;
            env.visuals.fog_color = glam::Vec3::from(params.fog_color);
        } else {
            env.visuals.fog_density = 0.0;
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

        // Override the camera UBO's light direction so the PBR shader uses the
        // world panel's sun settings instead of the internal TimeOfDay system.
        let dir = glam::Vec3::from(params.sun_dir).normalize();
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
