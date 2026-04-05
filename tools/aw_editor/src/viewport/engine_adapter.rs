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
        })
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn update_camera(&mut self, camera: &OrbitCamera) {
        let engine_camera = camera.to_engine_camera();
        self.renderer.update_camera(&engine_camera);
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

        for (i, (vertices, indices)) in chunks.iter().enumerate() {
            if vertices.is_empty() || indices.is_empty() {
                continue;
            }
            let name = format!("terrain_chunk_{i}");

            // Convert editor TerrainVertex → engine-compatible arrays.
            // The engine's create_mesh_from_full_arrays accepts positions, normals,
            // tangents, UVs, and indices separately.
            let positions: Vec<[f32; 3]> = vertices.iter().map(|v| v.position).collect();
            let normals: Vec<[f32; 3]> = vertices.iter().map(|v| v.normal).collect();
            let uvs: Vec<[f32; 2]> = vertices.iter().map(|v| v.uv).collect();
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

            let mesh = self
                .renderer
                .create_mesh_from_full_arrays(&positions, &normals, &tangents, &uvs, indices);

            let instance = astraweave_render::Instance::from_pos_scale_color(
                glam::Vec3::ZERO,
                glam::Vec3::ONE,
                [1.0, 1.0, 1.0, 1.0],
            );

            self.renderer.add_model(&name, mesh, &[instance]);
            self.terrain_model_names.push(name);
        }

        // Track stats for the scene stats panel
        self.terrain_total_indices = chunks.iter().map(|(_, idx)| idx.len()).sum();
        self.terrain_total_triangles = self.terrain_total_indices / 3;

        tracing::debug!(
            "Uploaded {} terrain chunks ({} triangles) to engine renderer",
            self.terrain_model_names.len(),
            self.terrain_total_triangles,
        );
    }

    /// Clear all terrain data from the engine renderer.
    pub fn clear_terrain(&mut self) {
        for name in self.terrain_model_names.drain(..) {
            self.renderer.clear_model(&name);
        }
        self.terrain_total_triangles = 0;
        self.terrain_total_indices = 0;
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

        for (key, items) in &groups {
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

            // If the model doesn't exist yet, create a placeholder mesh.
            // In the full pipeline, the mesh would be loaded from the glTF path.
            if !self.renderer.has_model(&name) {
                // Use a unit cube as placeholder — the actual glTF loading happens
                // through load_gltf_model when assets are available.
                let mesh = self.renderer.create_mesh_from_arrays(
                    &[
                        [-0.5, -0.5, -0.5],
                        [0.5, -0.5, -0.5],
                        [0.5, 0.5, -0.5],
                        [-0.5, 0.5, -0.5],
                        [-0.5, -0.5, 0.5],
                        [0.5, -0.5, 0.5],
                        [0.5, 0.5, 0.5],
                        [-0.5, 0.5, 0.5],
                    ],
                    &[
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, -1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                        [0.0, 0.0, 1.0],
                    ],
                    &[
                        0, 1, 2, 2, 3, 0, // front
                        4, 6, 5, 6, 4, 7, // back
                        0, 3, 7, 7, 4, 0, // left
                        1, 5, 6, 6, 2, 1, // right
                        3, 2, 6, 6, 7, 3, // top
                        0, 4, 5, 5, 1, 0, // bottom
                    ],
                );
                self.renderer.add_model(&name, mesh, &instances);
            } else {
                self.renderer.update_instances(&instances);
            }

            self.scatter_model_names.push(name);
        }

        self.scatter_placement_count = placements.len();
        self.scatter_draw_call_count = groups.len() as u32;

        tracing::debug!(
            "Uploaded {} scatter groups ({} total placements) to engine renderer",
            groups.len(),
            placements.len()
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

    /// Apply lighting parameters to the engine's scene environment.
    pub fn set_lighting_params(&mut self, params: &TerrainLightingParams) {
        let env = self.renderer.scene_environment_mut();
        env.visuals.ambient_color = glam::Vec3::from(params.ambient_color);
        env.visuals.ambient_intensity = params.ambient_intensity;
        // Fog color can also be influenced by lighting for consistency
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

#[cfg(not(feature = "astraweave-render"))]
pub struct EngineRenderAdapter;

#[cfg(not(feature = "astraweave-render"))]
impl EngineRenderAdapter {
    pub fn is_initialized(&self) -> bool {
        false
    }
}
