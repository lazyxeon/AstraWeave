//! Viewport Renderer
//!
//! Coordinates multi-pass rendering pipeline for 3D viewport.
//! Renders in order: Grid → Entities → Gizmos → Selection Outline

#![allow(dead_code)]
//!
//! # Performance Budget
//!
//! Target: <10ms per frame @ 1080p (60 FPS)
//! - Clear: <0.1ms
//! - Grid: ~0.5ms
//! - Entities: ~8ms (scales with entity count)
//! - Gizmos: ~1ms
//!
//! # Architecture
//!
//! ViewportRenderer delegates to specialized sub-renderers:
//! - `GridRenderer`: Floor grid + axes
//! - `EntityRenderer`: World entities (instanced cube rendering)
//! - `GizmoRenderer`: Transform handles (translate/rotate/scale)
//! - `PhysicsDebugRenderer`: Collider wireframes

use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::debug;

use super::camera::OrbitCamera;
use super::entity_renderer::EntityRenderer;
use super::gizmo_renderer::GizmoRendererWgpu;
use super::grid_renderer::GridRenderer;
use super::physics_renderer::PhysicsDebugRenderer;
use super::skybox_renderer::SkyboxRenderer;
use super::rain_renderer::RainRenderer;
use super::terrain_renderer::TerrainRenderer;
use super::water_renderer::WaterRenderer;
use crate::gizmo::GizmoState;
use astraweave_core::{Entity, World};

#[cfg(feature = "astraweave-render")]
use super::engine_adapter::EngineRenderAdapter;

/// Viewport rendering coordinator
///
/// Manages GPU resources and coordinates multi-pass rendering pipeline.
///
/// # Lifecycle
///
/// 1. Create once during editor initialization
/// 2. Call `render()` every frame
/// 3. Call `resize()` when viewport changes size
/// 4. Automatically cleaned up on drop (RAII)
pub struct ViewportRenderer {
    /// wgpu device (GPU interface)
    device: Arc<wgpu::Device>,

    /// wgpu queue (command submission)
    queue: Arc<wgpu::Queue>,

    /// Sub-renderers
    grid_renderer: GridRenderer,
    skybox_renderer: SkyboxRenderer,
    entity_renderer: EntityRenderer,
    gizmo_renderer: GizmoRendererWgpu,
    physics_renderer: PhysicsDebugRenderer,
    terrain_renderer: TerrainRenderer,
    water_renderer: WaterRenderer,
    rain_renderer: RainRenderer,

    /// Engine renderer adapter for PBR mesh rendering (feature-gated)
    #[cfg(feature = "astraweave-render")]
    engine_adapter: Option<EngineRenderAdapter>,

    /// Enable engine rendering (PBR meshes) vs cube rendering
    use_engine_rendering: bool,

    /// Depth texture (shared across passes)
    depth_texture: Option<wgpu::Texture>,

    /// Depth texture view
    depth_view: Option<wgpu::TextureView>,

    /// Current viewport size
    size: (u32, u32),

    /// Currently selected entities (for highlighting) - supports multi-selection
    selected_entities: Vec<Entity>,
}

impl ViewportRenderer {
    /// Create new viewport renderer
    ///
    /// # Arguments
    ///
    /// * `device` - wgpu device
    /// * `queue` - wgpu queue
    ///
    /// # Errors
    ///
    /// Returns error if sub-renderer creation fails.
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        let grid_renderer = GridRenderer::new(&device).context("Failed to create grid renderer")?;
        let skybox_renderer =
            SkyboxRenderer::new(&device).context("Failed to create skybox renderer")?;
        let entity_renderer =
            EntityRenderer::new(device.clone(), 10000).context("Failed to create entity renderer")?;
        let gizmo_renderer = GizmoRendererWgpu::new((*device).clone(), (*queue).clone(), 10000)
            .context("Failed to create gizmo renderer")?;
        let physics_renderer =
            PhysicsDebugRenderer::new((*device).clone(), (*queue).clone(), 50000)
                .context("Failed to create physics debug renderer")?;
        let terrain_renderer =
            TerrainRenderer::new(&device).context("Failed to create terrain renderer")?;
        let water_renderer =
            WaterRenderer::new(&device).context("Failed to create water renderer")?;
        let rain_renderer =
            RainRenderer::new(&device).context("Failed to create rain renderer")?;

        Ok(Self {
            device,
            queue,
            grid_renderer,
            skybox_renderer,
            entity_renderer,
            gizmo_renderer,
            physics_renderer,
            terrain_renderer,
            water_renderer,
            rain_renderer,
            #[cfg(feature = "astraweave-render")]
            engine_adapter: None,
            use_engine_rendering: false,
            depth_texture: None,
            depth_view: None,
            size: (0, 0),
            selected_entities: Vec::new(),
        })
    }

    /// Create from eframe render state
    ///
    /// # Arguments
    ///
    /// * `render_state` - eframe's wgpu render state
    ///
    /// # Errors
    ///
    /// Returns error if render state is invalid or sub-renderer creation fails.
    pub fn from_eframe(render_state: &eframe::egui_wgpu::RenderState) -> Result<Self> {
        let device = Arc::new(render_state.device.clone());
        let queue = Arc::new(render_state.queue.clone());
        Self::new(device, queue)
    }

    /// Resize viewport (recreates depth buffer)
    ///
    /// Call this when viewport dimensions change.
    ///
    /// # Arguments
    ///
    /// * `width` - New width (pixels)
    /// * `height` - New height (pixels)
    ///
    /// # Errors
    ///
    /// Returns error if depth buffer creation fails.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            // Invalid size, clear depth buffer
            self.depth_texture = None;
            self.depth_view = None;
            self.size = (0, 0);
            return Ok(());
        }

        // Create depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
        self.size = (width, height);

        #[cfg(feature = "astraweave-render")]
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.resize(width, height);
        }

        Ok(())
    }

    /// Render the 3D scene
    ///
    /// Multi-pass rendering pipeline:
    /// 1. Skybox pass (clears depth and renders background gradient)
    /// 2. Grid pass (render floor grid)
    /// 3. Entity pass (render all world entities)
    /// 4. Physics debug pass (collider wireframes if enabled)
    /// 5. Gizmo pass (render transform gizmos if entity selected)
    ///
    /// # Arguments
    ///
    /// * `target` - Render target texture
    /// * `camera` - Camera for view-projection
    /// * `world` - Entity world state
    /// * `gizmo_state` - Optional gizmo state (for transform operations)
    /// * `hovered_axis` - Currently hovered axis for gizmo highlighting
    /// * `physics_debug_lines` - Optional physics debug lines from PhysicsWorld
    /// * `show_grid` - Whether to render the grid at all
    /// * `crosshair_mode` - If true, render only axis lines (crosshair), not full grid
    /// * `shading_mode` - 0=Lit, 1=Unlit, 2=Wireframe
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        target: &wgpu::Texture,
        camera: &OrbitCamera,
        world: &World,
        gizmo_state: Option<&GizmoState>,
        hovered_axis: Option<crate::gizmo::AxisConstraint>,
        physics_debug_lines: Option<&[astraweave_physics::DebugLine]>,
        show_grid: bool,
        crosshair_mode: bool,
        shading_mode: u32,
    ) -> Result<()> {
        // Ensure depth buffer matches target size
        let target_size = target.size();
        if self.size != (target_size.width, target_size.height) {
            self.resize(target_size.width, target_size.height)
                .context("Failed to resize depth buffer")?;
        }

        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = self
            .depth_view
            .as_ref()
            .context("Depth buffer not initialized")?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Viewport Render Encoder"),
            });

        // Pass 1: Skybox (clears color/depth and renders gradient background)
        self.skybox_renderer
            .render(&mut encoder, &target_view, depth_view, camera, &self.queue)
            .context("Skybox render failed")?;

        // Pass 2: Grid (only if enabled)
        if show_grid {
            self.grid_renderer
                .render(
                    &mut encoder,
                    &target_view,
                    depth_view,
                    camera,
                    &self.queue,
                    crosshair_mode,
                )
                .context("Grid render failed")?;
        }

        // Pass 2.5: Terrain (generated terrain chunks)
        self.terrain_renderer
            .render(
                &mut encoder,
                &target_view,
                depth_view,
                camera,
                &self.queue,
                shading_mode,
            )
            .context("Terrain render failed")?;

        // Pass 2.7: Water plane (transparent, rendered after terrain)
        self.water_renderer
            .render(&mut encoder, &target_view, depth_view, camera, &self.queue)
            .context("Water render failed")?;

        // Pass 3: Entities (engine renderer or cube fallback)
        #[cfg(feature = "astraweave-render")]
        {
            if self.use_engine_rendering {
                if let Some(adapter) = &mut self.engine_adapter {
                    adapter.update_camera(camera);
                    adapter
                        .render_to_texture(&target_view, &mut encoder)
                        .context("Engine render failed")?;
                } else {
                    self.entity_renderer
                        .render(
                            &mut encoder,
                            &target_view,
                            depth_view,
                            camera,
                            world,
                            &self.selected_entities,
                            &self.queue,
                            shading_mode,
                        )
                        .context("Entity render failed")?;
                }
            } else {
                self.entity_renderer
                    .render(
                        &mut encoder,
                        &target_view,
                        depth_view,
                        camera,
                        world,
                        &self.selected_entities,
                        &self.queue,
                        shading_mode,
                    )
                    .context("Entity render failed")?;
            }
        }
        #[cfg(not(feature = "astraweave-render"))]
        {
            self.entity_renderer
                .render(
                    &mut encoder,
                    &target_view,
                    depth_view,
                    camera,
                    world,
                    &self.selected_entities,
                    &self.queue,
                    shading_mode,
                )
                .context("Entity render failed")?;
        }

        // Pass 4: Physics debug (collider wireframes)
        if let Some(debug_lines) = physics_debug_lines {
            self.physics_renderer
                .render(&mut encoder, &target_view, depth_view, camera, debug_lines)
                .context("Physics debug render failed")?;
        }

        // Pass 4.5: Rain particles (transparent volumetric rain)
        self.rain_renderer
            .render(&mut encoder, &target_view, depth_view, camera, &self.queue)
            .context("Rain render failed")?;

        // Pass 5: Gizmos (if entity selected and gizmo active)
        if let (Some(selected), Some(gizmo)) = (self.selected_entity(), gizmo_state) {
            if gizmo.mode != crate::gizmo::GizmoMode::Inactive {
                // DEBUG: Log gizmo mode and constraint
                if let crate::gizmo::GizmoMode::Rotate { constraint } = &gizmo.mode {
                    debug!(
                        "Renderer: Rendering Rotate gizmo, constraint = {:?}",
                        constraint
                    );
                }

                // Get entity position from world (old astraweave-core API)
                if let Some(pose) = world.pose(selected) {
                    // Convert astraweave_core::IVec2 to glam::IVec2
                    let glam_pos = glam::IVec2::new(pose.pos.x, pose.pos.y);

                    self.gizmo_renderer
                        .render(
                            &mut encoder,
                            &target_view,
                            depth_view,
                            camera,
                            gizmo,
                            glam_pos,
                            hovered_axis,
                            &self.queue,
                        )
                        .context("Gizmo render failed")?;
                }
            }
        }

        // Submit all commands
        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    /// Create render texture
    ///
    /// Creates a texture suitable for rendering to and displaying in egui.
    ///
    /// # Arguments
    ///
    /// * `width` - Texture width (pixels)
    /// * `height` - Texture height (pixels)
    ///
    /// # Returns
    ///
    /// Texture with RENDER_ATTACHMENT | TEXTURE_BINDING usage.
    pub fn create_render_texture(&self, width: u32, height: u32) -> Result<wgpu::Texture> {
        if width == 0 || height == 0 {
            anyhow::bail!("Invalid texture size: {}x{}", width, height);
        }

        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }))
    }

    /// Get current viewport size
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Get wgpu device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get wgpu queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Set selected entities (for highlighting) - supports multi-selection
    pub fn set_selected_entities(&mut self, entities: &[Entity]) {
        self.selected_entities = entities.to_vec();
    }

    /// Set the entity-to-mesh mapping so models render with actual GLTF geometry
    pub fn set_entity_meshes(&mut self, meshes: std::collections::HashMap<Entity, String>) {
        self.entity_renderer.set_entity_meshes(meshes);
    }

    /// Set selected entity (for backward compatibility)
    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        self.selected_entities.clear();
        if let Some(e) = entity {
            self.selected_entities.push(e);
        }
    }

    /// Get selected entity (returns first selected for backward compatibility)
    pub fn selected_entity(&self) -> Option<Entity> {
        self.selected_entities.first().copied()
    }

    /// Get all selected entities
    pub fn selected_entities(&self) -> &[Entity] {
        &self.selected_entities
    }

    /// Handle GPU device lost
    ///
    /// Clears all GPU-dependent resources and prepares for recovery.
    pub fn handle_device_lost(&mut self) -> Result<()> {
        tracing::error!("GPU device lost in ViewportRenderer - cleaning up resources for recovery");

        // Clear resources that depend on the old device
        self.depth_texture = None;
        self.depth_view = None;

        #[cfg(feature = "astraweave-render")]
        {
            self.engine_adapter = None;
        }

        // Sub-renderers may need to be recreated with a new device too
        // but that usually happens by recreating the ViewportRenderer itself.

        Ok(())
    }

    /// Get physics debug options (mutable) for configuration
    pub fn physics_debug_options_mut(
        &mut self,
    ) -> &mut super::physics_renderer::PhysicsDebugOptions {
        &mut self.physics_renderer.options
    }

    pub fn upload_terrain_chunks(
        &mut self,
        chunks: &[(Vec<super::terrain_renderer::TerrainVertex>, Vec<u32>)],
    ) {
        self.terrain_renderer.upload_chunks(chunks);
    }

    pub fn clear_terrain(&mut self) {
        self.terrain_renderer.clear_chunks();
    }

    pub fn terrain_chunk_count(&self) -> usize {
        self.terrain_renderer.chunk_count()
    }

    /// Check if physics debug rendering is enabled
    pub fn physics_debug_enabled(&self) -> bool {
        self.physics_renderer.options.show_colliders
    }

    /// Enable/disable physics debug rendering
    pub fn set_physics_debug_enabled(&mut self, enabled: bool) {
        self.physics_renderer.options.show_colliders = enabled;
    }

    /// Load an HDRI file and apply it as the skybox background
    pub fn load_hdri(&mut self, path: &std::path::Path) -> Result<()> {
        self.skybox_renderer
            .load_hdri(&self.device, &self.queue, path)
            .context("Failed to load HDRI skybox")
    }

    /// Remove the HDRI skybox and revert to procedural gradient
    pub fn clear_hdri(&mut self) {
        self.skybox_renderer.clear_hdri();
    }

    /// Set environment sky colors (for skybox presets, time-of-day, weather)
    pub fn set_sky_colors(
        &mut self,
        sky_top: [f32; 4],
        sky_horizon: [f32; 4],
        ground_color: [f32; 4],
    ) {
        self.skybox_renderer
            .set_sky_colors(sky_top, sky_horizon, ground_color);
    }

    /// Set fog and weather parameters for distance-based terrain fog
    pub fn set_fog_params(&mut self, params: super::terrain_renderer::TerrainFogParams) {
        self.terrain_renderer.set_fog_params(params);
        // Forward fog to water renderer
        self.water_renderer
            .set_fog(params.fog_enabled, params.fog_density, params.fog_color);
        // Enable rain for weather types 2 (Rain) and 3 (Storm)
        let rain_active = params.weather_type == 2 || params.weather_type == 3;
        let rain_intensity = if params.weather_type == 3 { 1.0 } else { 0.5 };
        self.rain_renderer.set_active(rain_active, rain_intensity);
        // Wind for storm
        let (wx, wz) = match params.weather_type {
            3 => (4.0, 2.0),
            2 => (1.0, 0.5),
            _ => (0.0, 0.0),
        };
        self.rain_renderer.set_wind(wx, wz);
    }

    /// Set water level for volumetric water plane
    pub fn set_water_level(&mut self, level: f32) {
        self.water_renderer.set_water_level(level);
        self.terrain_renderer.set_water_level(level);
    }

    /// Check if engine rendering (PBR meshes) is enabled
    pub fn use_engine_rendering(&self) -> bool {
        self.use_engine_rendering
    }

    /// Enable/disable engine rendering (PBR meshes vs cubes)
    pub fn set_use_engine_rendering(&mut self, enabled: bool) {
        self.use_engine_rendering = enabled;
    }

    /// Initialize the engine renderer adapter (async, call once)
    ///
    /// Must be called before engine rendering can be used.
    /// Uses the viewport's current size for initialization.
    #[cfg(feature = "astraweave-render")]
    pub async fn init_engine_adapter(&mut self) -> Result<()> {
        if self.engine_adapter.is_some() {
            return Ok(());
        }

        let (width, height) = if self.size.0 > 0 && self.size.1 > 0 {
            self.size
        } else {
            (1920, 1080)
        };

        let adapter =
            EngineRenderAdapter::new(self.device.clone(), self.queue.clone(), width, height)
                .await
                .context("Failed to initialize engine render adapter")?;

        self.engine_adapter = Some(adapter);
        Ok(())
    }

    /// Check if engine adapter is initialized
    #[cfg(feature = "astraweave-render")]
    pub fn engine_adapter_initialized(&self) -> bool {
        self.engine_adapter.is_some()
    }

    /// Get immutable reference to engine adapter (if initialized)
    #[cfg(feature = "astraweave-render")]
    pub fn engine_adapter(&self) -> Option<&EngineRenderAdapter> {
        self.engine_adapter.as_ref()
    }

    /// Get mutable reference to engine adapter (if initialized)
    #[cfg(feature = "astraweave-render")]
    pub fn engine_adapter_mut(&mut self) -> Option<&mut EngineRenderAdapter> {
        self.engine_adapter.as_mut()
    }
}

impl Drop for ViewportRenderer {
    fn drop(&mut self) {
        tracing::debug!(
            "Dropping ViewportRenderer - cleaning up GPU resources (depth_texture: {})",
            self.depth_texture.is_some()
        );
        // Explicitly clear optional resources
        self.depth_texture = None;
        self.depth_view = None;
        #[cfg(feature = "astraweave-render")]
        {
            self.engine_adapter = None;
        }
    }
}

#[cfg(test)]
mod tests {
    // NOTE: These tests require wgpu device, which needs a GPU or software renderer.
    // Run with: cargo test --features gpu-tests

    #[test]
    fn test_viewport_renderer_resize() {
        // This is a smoke test - just ensure no panics
        // Actual GPU tests would require a wgpu instance
    }
}
