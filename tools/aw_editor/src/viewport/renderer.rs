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
use super::scatter_renderer::{ScatterPlacement, ScatterRenderer};
use super::skybox_renderer::SkyboxRenderer;
use super::terrain_renderer::TerrainRenderer;
use super::water_renderer::WaterRenderer;
use super::weather_particle_renderer::{WeatherKind, WeatherParticleRenderer};
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

    /// Sub-renderers (essential — created at startup)
    grid_renderer: GridRenderer,
    skybox_renderer: SkyboxRenderer,
    entity_renderer: EntityRenderer,
    gizmo_renderer: GizmoRendererWgpu,

    /// Sub-renderers (deferred — created lazily on first use)
    physics_renderer: Option<PhysicsDebugRenderer>,
    terrain_renderer: Option<TerrainRenderer>,
    water_renderer: Option<WaterRenderer>,
    weather_renderer: Option<WeatherParticleRenderer>,
    scatter_renderer: Option<ScatterRenderer>,

    /// Scatter placements for current frame
    scatter_placements: Vec<ScatterPlacement>,

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

    // --- Depth readback for brush hit detection (deferred 1-frame) ---
    /// Staging buffer for reading back a single depth pixel
    depth_staging_buffer: Option<wgpu::Buffer>,
    /// True when an async depth read is in-flight (waiting for GPU)
    depth_read_pending: bool,
    /// Set to true by map_async callback when GPU finishes the depth copy
    depth_map_ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Cached depth value from previous frame's readback
    cached_depth_value: Option<f32>,

    /// Currently selected entities (for highlighting) - supports multi-selection
    selected_entities: Vec<Entity>,

    /// Component gizmo debug lines (light radius, collider shapes, audio range)
    component_gizmo_lines: Vec<astraweave_physics::DebugLine>,

    /// Brush cursor circle draped on terrain surface
    brush_cursor_lines: Vec<astraweave_physics::DebugLine>,

    /// Zone overlay lines (blueprint zone wireframes)
    zone_overlay_lines: Vec<astraweave_physics::DebugLine>,
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
        // Only create essential renderers at startup (grid, skybox, entities, gizmos).
        // Non-essential renderers (terrain, water, rain, scatter, physics debug)
        // are deferred to first use to minimize time-to-first-frame.
        let grid_renderer = GridRenderer::new(&device).context("Failed to create grid renderer")?;
        let skybox_renderer =
            SkyboxRenderer::new(&device).context("Failed to create skybox renderer")?;
        let entity_renderer = EntityRenderer::new(device.clone(), queue.clone(), 10000)
            .context("Failed to create entity renderer")?;
        let gizmo_renderer = GizmoRendererWgpu::new((*device).clone(), (*queue).clone(), 10000)
            .context("Failed to create gizmo renderer")?;

        Ok(Self {
            device,
            queue,
            grid_renderer,
            skybox_renderer,
            entity_renderer,
            gizmo_renderer,
            physics_renderer: None,
            terrain_renderer: None,
            water_renderer: None,
            weather_renderer: None,
            scatter_renderer: None,
            scatter_placements: Vec::new(),
            #[cfg(feature = "astraweave-render")]
            engine_adapter: None,
            use_engine_rendering: false,
            depth_texture: None,
            depth_view: None,
            size: (0, 0),
            depth_staging_buffer: None,
            depth_read_pending: false,
            depth_map_ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cached_depth_value: None,
            selected_entities: Vec::new(),
            component_gizmo_lines: Vec::new(),
            brush_cursor_lines: Vec::new(),
            zone_overlay_lines: Vec::new(),
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

    // ── Deferred renderer lazy-init helpers ─────────────────────────────

    fn ensure_terrain_renderer(&mut self) -> Result<&mut TerrainRenderer> {
        if self.terrain_renderer.is_none() {
            self.terrain_renderer = Some(
                TerrainRenderer::new(&self.device, &self.queue)
                    .context("Failed to create terrain renderer (deferred)")?,
            );
        }
        self.terrain_renderer
            .as_mut()
            .context("terrain renderer not initialized after creation")
    }

    fn ensure_water_renderer(&mut self) -> Result<&mut WaterRenderer> {
        if self.water_renderer.is_none() {
            self.water_renderer = Some(
                WaterRenderer::new(&self.device)
                    .context("Failed to create water renderer (deferred)")?,
            );
        }
        self.water_renderer
            .as_mut()
            .context("water renderer not initialized after creation")
    }

    fn ensure_weather_renderer(&mut self) -> Result<&mut WeatherParticleRenderer> {
        if self.weather_renderer.is_none() {
            self.weather_renderer = Some(
                WeatherParticleRenderer::new(&self.device)
                    .context("Failed to create weather particle renderer (deferred)")?,
            );
        }
        self.weather_renderer
            .as_mut()
            .context("weather renderer not initialized after creation")
    }

    fn ensure_scatter_renderer(&mut self) -> Result<&mut ScatterRenderer> {
        if self.scatter_renderer.is_none() {
            self.scatter_renderer = Some(
                ScatterRenderer::new(self.device.clone(), self.queue.clone())
                    .context("Failed to create scatter renderer (deferred)")?,
            );
        }
        self.scatter_renderer
            .as_mut()
            .context("scatter renderer not initialized after creation")
    }

    fn ensure_physics_renderer(&mut self) -> Result<&mut PhysicsDebugRenderer> {
        if self.physics_renderer.is_none() {
            self.physics_renderer = Some(
                PhysicsDebugRenderer::new((*self.device).clone(), (*self.queue).clone(), 5000)
                    .context("Failed to create physics debug renderer (deferred)")?,
            );
        }
        self.physics_renderer
            .as_mut()
            .context("physics renderer not initialized after creation")
    }

    /// Eagerly initialize all deferred renderers to avoid frame hitches during gameplay.
    /// Call once after the first frame has rendered (GPU device is warm).
    pub fn eagerly_init_all(&mut self) {
        if self.terrain_renderer.is_none() {
            match TerrainRenderer::new(&self.device, &self.queue) {
                Ok(r) => self.terrain_renderer = Some(r),
                Err(e) => tracing::warn!("Eager init terrain renderer failed: {e:#}"),
            }
        }
        if self.water_renderer.is_none() {
            match WaterRenderer::new(&self.device) {
                Ok(r) => self.water_renderer = Some(r),
                Err(e) => tracing::warn!("Eager init water renderer failed: {e:#}"),
            }
        }
        if self.weather_renderer.is_none() {
            match WeatherParticleRenderer::new(&self.device) {
                Ok(r) => self.weather_renderer = Some(r),
                Err(e) => tracing::warn!("Eager init weather renderer failed: {e:#}"),
            }
        }
        if self.scatter_renderer.is_none() {
            match ScatterRenderer::new(self.device.clone(), self.queue.clone()) {
                Ok(r) => self.scatter_renderer = Some(r),
                Err(e) => tracing::warn!("Eager init scatter renderer failed: {e:#}"),
            }
        }
        if self.physics_renderer.is_none() {
            match PhysicsDebugRenderer::new((*self.device).clone(), (*self.queue).clone(), 5000) {
                Ok(r) => self.physics_renderer = Some(r),
                Err(e) => tracing::warn!("Eager init physics renderer failed: {e:#}"),
            }
        }
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.depth_texture = Some(depth_texture);
        self.depth_view = Some(depth_view);
        self.size = (width, height);

        // Cancel any in-flight depth readback before replacing the staging buffer
        self.depth_read_pending = false;
        self.depth_map_ready
            .store(false, std::sync::atomic::Ordering::Release);

        // Allocate staging buffer for single-pixel depth readback (4 bytes = f32)
        // wgpu requires COPY_DST on staging buffers used as copy destinations.
        // Row alignment: bytes_per_row must be multiple of 256 for copy_texture_to_buffer,
        // so we allocate 256 bytes even though we only read 4.
        self.depth_staging_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Depth Readback Staging"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));

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

        // Eagerly init scatter renderer before depth_view borrow (avoids self re-borrow)
        if !self.scatter_placements.is_empty() && self.scatter_renderer.is_none() {
            let _ = self.ensure_scatter_renderer();
        }

        // Eagerly init physics renderer if component gizmo lines, physics lines, brush cursor, or zone overlay exist
        if (!self.component_gizmo_lines.is_empty()
            || physics_debug_lines.is_some()
            || !self.brush_cursor_lines.is_empty()
            || !self.zone_overlay_lines.is_empty())
            && self.physics_renderer.is_none()
        {
            let _ = self.ensure_physics_renderer();
        }

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

        // Pass 2.5: Terrain (generated terrain chunks) — deferred init
        if let Some(terrain) = self.terrain_renderer.as_mut() {
            terrain
                .render(
                    &mut encoder,
                    &target_view,
                    depth_view,
                    camera,
                    &self.queue,
                    shading_mode,
                )
                .context("Terrain render failed")?;
        }

        // Pass 2.6: Scatter objects (GPU-instanced vegetation, rocks, props) — deferred init
        if !self.scatter_placements.is_empty() {
            if let Some(scatter) = self.scatter_renderer.as_mut() {
                // Take placements temporarily (borrow-checker requires this since
                // scatter.render() borrows &mut self through scatter_renderer).
                // CRITICAL: Always restore placements before propagating errors.
                let placements = std::mem::take(&mut self.scatter_placements);
                let result = scatter.render(
                    &mut encoder,
                    &target_view,
                    depth_view,
                    camera,
                    &placements,
                    &self.queue,
                );
                self.scatter_placements = placements;
                result.context("Scatter render failed")?;
            }
        }

        // Pass 2.7: Water plane (transparent, rendered after terrain) — deferred init
        if let Some(water) = self.water_renderer.as_mut() {
            if water.is_enabled() {
                water
                    .render(&mut encoder, &target_view, depth_view, camera, &self.queue)
                    .context("Water render failed")?;
            }
        }

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

        // Pass 4: Physics debug + component gizmos + brush cursor
        {
            let phys_lines = physics_debug_lines.unwrap_or(&[]);
            let total_lines = self.component_gizmo_lines.len()
                + phys_lines.len()
                + self.brush_cursor_lines.len()
                + self.zone_overlay_lines.len();
            if total_lines > 0 {
                if let Some(physics) = self.physics_renderer.as_mut() {
                    // Pre-allocate exact capacity to avoid reallocation churn
                    let mut combined_lines = Vec::with_capacity(total_lines);
                    combined_lines.extend_from_slice(&self.component_gizmo_lines);
                    combined_lines.extend_from_slice(phys_lines);
                    combined_lines.extend_from_slice(&self.brush_cursor_lines);
                    combined_lines.extend_from_slice(&self.zone_overlay_lines);
                    physics
                        .render(
                            &mut encoder,
                            &target_view,
                            depth_view,
                            camera,
                            &combined_lines,
                        )
                        .context("Physics debug render failed")?;
                }
            }
        }

        // Pass 4.5: Weather particles (rain, snow, hail, sandstorm, blizzard) — deferred init
        if let Some(weather) = self.weather_renderer.as_mut() {
            weather
                .render(&mut encoder, &target_view, depth_view, camera, &self.queue)
                .context("Weather particle render failed")?;
        }

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
                            pose.height,
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
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// Get wgpu queue
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Set selected entities (for highlighting) - supports multi-selection
    pub fn set_selected_entities(&mut self, entities: &[Entity]) {
        self.selected_entities = entities.to_vec();
    }

    /// Set component gizmo debug lines (light radii, collider shapes, audio ranges).
    /// These are rendered in the physics debug pass.
    pub fn set_component_gizmo_lines(&mut self, lines: Vec<astraweave_physics::DebugLine>) {
        self.component_gizmo_lines = lines;
    }

    pub fn set_brush_cursor_lines(&mut self, lines: Vec<astraweave_physics::DebugLine>) {
        self.brush_cursor_lines = lines;
    }

    /// Set zone overlay lines (blueprint zone wireframes)
    pub fn set_zone_overlay_lines(&mut self, lines: Vec<astraweave_physics::DebugLine>) {
        self.zone_overlay_lines = lines;
    }

    /// Set the entity-to-mesh mapping so models render with actual GLTF geometry
    pub fn set_entity_meshes(&mut self, meshes: std::collections::HashMap<Entity, String>) {
        self.entity_renderer.set_entity_meshes(meshes);
    }

    /// Set per-entity external texture overrides (entity → texture file path).
    pub fn set_entity_texture_overrides(
        &mut self,
        overrides: std::collections::HashMap<Entity, String>,
    ) {
        self.entity_renderer.set_entity_texture_overrides(overrides);
    }

    /// Get skeleton for a mesh (delegates to entity renderer).
    pub fn get_mesh_skeleton(
        &self,
        mesh_path: &str,
    ) -> Option<&super::entity_renderer::GltfSkeleton> {
        self.entity_renderer.get_mesh_skeleton(mesh_path)
    }

    /// Get animation clips for a mesh (delegates to entity renderer).
    pub fn get_mesh_animations(
        &self,
        mesh_path: &str,
    ) -> &[super::entity_renderer::GltfAnimationClip] {
        self.entity_renderer.get_mesh_animations(mesh_path)
    }

    /// Apply CPU skinning to a mesh (delegates to entity renderer).
    pub fn apply_cpu_skinning(&mut self, mesh_path: &str, joint_matrices: &[glam::Mat4]) {
        self.entity_renderer
            .apply_cpu_skinning(mesh_path, joint_matrices, &self.queue);
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

    /// Get physics debug options (mutable) for configuration — lazily inits renderer
    pub fn physics_debug_options_mut(
        &mut self,
    ) -> Result<&mut super::physics_renderer::PhysicsDebugOptions> {
        self.ensure_physics_renderer()?;
        Ok(&mut self
            .physics_renderer
            .as_mut()
            .context("physics renderer not available")?
            .options)
    }

    pub fn upload_terrain_chunks(
        &mut self,
        chunks: &[(Vec<super::terrain_renderer::TerrainVertex>, Vec<u32>)],
    ) {
        if let Ok(terrain) = self.ensure_terrain_renderer() {
            terrain.upload_chunks(chunks);
        }
    }

    /// Upload terrain chunks using the terrain_integration vertex type directly
    /// (zero-copy path — avoids redundant field-by-field vertex remapping).
    pub fn upload_terrain_chunks_raw(
        &mut self,
        chunks: &[(Vec<crate::terrain_integration::TerrainVertex>, Vec<u32>)],
    ) {
        if let Ok(terrain) = self.ensure_terrain_renderer() {
            terrain.upload_chunks_raw(chunks);
        }
    }

    /// Incrementally update vertex data for a single terrain chunk on the GPU.
    pub fn update_terrain_chunk_vertices(
        &mut self,
        chunk_index: usize,
        vertices: &[super::terrain_renderer::TerrainVertex],
    ) {
        if let Some(terrain) = self.terrain_renderer.as_mut() {
            terrain.update_chunk_vertices(chunk_index, vertices);
        }
    }

    // ── Depth Readback for Brush Hit Detection ──────────────────────────

    /// Synchronously read the depth value at a single pixel from the depth buffer.
    /// Read depth at a pixel with 1-frame deferred readback.
    ///
    /// **Frame N**: Returns cached depth from frame N-1 and submits a new async read
    /// for the requested pixel. This eliminates the `device.poll(Wait)` GPU stall
    /// that previously blocked the CPU for 0.5-2ms every frame.
    ///
    /// Returns `Some(depth)` where depth is in [0,1] (0=near, 1=far/sky).
    /// Returns `None` on the first frame or if the depth buffer isn't available.
    pub fn read_depth_at_pixel(&mut self, px: u32, py: u32) -> Option<f32> {
        let (w, h) = self.size;
        if px >= w || py >= h {
            return None;
        }
        let depth_tex = self.depth_texture.as_ref()?;
        let staging = self.depth_staging_buffer.as_ref()?;

        // Try to read the result from the PREVIOUS frame's async request (non-blocking)
        let cached_depth = if self.depth_read_pending {
            // Non-blocking poll — let wgpu process pending callbacks
            let _ = self.device.poll(wgpu::MaintainBase::Poll);

            if self
                .depth_map_ready
                .load(std::sync::atomic::Ordering::Acquire)
            {
                // GPU finished — read the mapped buffer
                let mapped = staging.slice(..4).get_mapped_range();
                let depth_bytes: [u8; 4] = [mapped[0], mapped[1], mapped[2], mapped[3]];
                let depth = f32::from_le_bytes(depth_bytes);
                drop(mapped);
                staging.unmap();
                self.depth_read_pending = false;
                self.depth_map_ready
                    .store(false, std::sync::atomic::Ordering::Release);
                self.cached_depth_value = Some(depth);
                Some(depth)
            } else {
                // GPU not ready yet — use whatever we cached previously
                self.cached_depth_value
            }
        } else {
            self.cached_depth_value
        };

        // Submit a new async copy for THIS frame's pixel (result available next frame)
        if !self.depth_read_pending {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Depth Readback Encoder"),
                });

            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: depth_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: px, y: py, z: 0 },
                    aspect: wgpu::TextureAspect::DepthOnly,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: staging,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: None,
                    },
                },
                wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
            );

            self.queue.submit(std::iter::once(encoder.finish()));
            // Request async map — callback sets the atomic flag only on success
            let flag = self.depth_map_ready.clone();
            staging
                .slice(..4)
                .map_async(wgpu::MapMode::Read, move |result| {
                    if result.is_ok() {
                        flag.store(true, std::sync::atomic::Ordering::Release);
                    }
                });
            self.depth_read_pending = true;
        }

        cached_depth
    }

    pub fn clear_terrain(&mut self) {
        if let Some(terrain) = self.terrain_renderer.as_mut() {
            terrain.clear_chunks();
        }
    }

    pub fn terrain_chunk_count(&self) -> usize {
        self.terrain_renderer
            .as_ref()
            .map_or(0, |t| t.chunk_count())
    }

    /// Check if physics debug rendering is enabled
    pub fn physics_debug_enabled(&self) -> bool {
        self.physics_renderer
            .as_ref()
            .map_or(false, |p| p.options.show_colliders)
    }

    /// Enable/disable physics debug rendering
    pub fn set_physics_debug_enabled(&mut self, enabled: bool) {
        if let Ok(physics) = self.ensure_physics_renderer() {
            physics.options.show_colliders = enabled;
        }
    }

    /// Check if any animated weather effects are active (rain, etc.)
    pub fn has_active_effects(&self) -> bool {
        self.weather_renderer
            .as_ref()
            .map_or(false, |r| r.is_active())
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
        if let Ok(terrain) = self.ensure_terrain_renderer() {
            terrain.set_fog_params(params);
        }
        // Forward fog to water renderer only if it already exists (don't create it)
        if let Some(water) = self.water_renderer.as_mut() {
            water.set_fog(params.fog_enabled, params.fog_density, params.fog_color);
        }
        // Forward fog to scatter renderer only if it already exists (don't create it)
        if let Some(scatter) = self.scatter_renderer.as_mut() {
            scatter.set_fog_params(params);
        }
        // Map weather type to particle weather kind — only create renderer if weather is active
        let kind = WeatherKind::from_weather_type(params.weather_type);
        if kind != WeatherKind::None {
            let intensity = match params.weather_type {
                3 => 1.0, // Storm = heavy
                _ => 0.7,
            };
            let queue = self.queue.clone();
            if let Ok(weather) = self.ensure_weather_renderer() {
                weather.set_particle_count_override(params.particle_count_override);
                weather.set_weather(kind, intensity, &queue);
                let (wx, wz) = match params.weather_type {
                    3 => (5.0, 3.0),
                    2 => (1.5, 0.8),
                    6 => (6.0, 3.0), // Sandstorm = strong wind
                    _ => (0.0, 0.0),
                };
                weather.set_wind(wx, wz);
            }
        } else if let Some(weather) = self.weather_renderer.as_mut() {
            // Weather off — deactivate existing renderer without creating one
            let queue = self.queue.clone();
            weather.set_weather(WeatherKind::None, 0.0, &queue);
        }
    }

    /// Set lighting parameters for PBR terrain shading (also syncs to entity renderer)
    pub fn set_lighting_params(&mut self, params: super::terrain_renderer::TerrainLightingParams) {
        if let Ok(terrain) = self.ensure_terrain_renderer() {
            terrain.set_lighting_params(params);
        }
        // Forward lighting to scatter renderer only if it already exists
        if let Some(scatter) = self.scatter_renderer.as_mut() {
            scatter.set_lighting_params(params);
        }
        // Sync sun/ambient to entity renderer so entities share the same directional light
        self.entity_renderer
            .set_sun(params.sun_dir, params.sun_color, params.sun_intensity);
        self.entity_renderer
            .set_ambient(params.ambient_color, params.ambient_intensity);
        // Forward sun direction to water renderer for consistent specular reflections
        if let Some(water) = self.water_renderer.as_mut() {
            water.set_sun(params.sun_dir, params.sun_intensity);
        }
    }

    /// Set scene point lights from entity Light components (forwarded to entity renderer)
    pub fn set_scene_lights(&mut self, lights: Vec<super::entity_renderer::SceneLight>) {
        self.entity_renderer.set_scene_lights(lights);
    }

    /// Set water level for volumetric water plane
    pub fn set_water_level(&mut self, level: f32) {
        // Only forward to water renderer if it already exists (don't create it)
        if let Some(water) = self.water_renderer.as_mut() {
            water.set_water_level(level);
        }
        if let Ok(terrain) = self.ensure_terrain_renderer() {
            terrain.set_water_level(level);
        }
    }

    /// Enable or disable the volumetric water plane
    pub fn set_water_enabled(&mut self, enabled: bool) {
        if enabled {
            if let Ok(water) = self.ensure_water_renderer() {
                water.set_enabled(true);
            }
        } else {
            // Drop the water renderer entirely to guarantee no rendering
            self.water_renderer = None;
        }
    }

    // ── Scatter management ──────────────────────────────────────────────

    /// Set scatter placements for instanced rendering.
    pub fn set_scatter_placements(&mut self, placements: Vec<ScatterPlacement>) {
        tracing::info!(
            "Renderer: set_scatter_placements({} items)",
            placements.len()
        );
        if let Some(sample) = placements.first() {
            tracing::info!(
                "Renderer scatter sample: key='{}' path='{}'",
                sample.mesh_key,
                sample.mesh_path
            );
        }
        self.scatter_placements = placements;
    }

    // ── Terrain texture management ────────────────────────────────────

    /// Replace a single material layer's textures in the terrain renderer.
    /// Used to inject BiomePack ground textures into the GPU texture arrays.
    /// `layer_index` should be 12-17 (reserved custom pack slots).
    pub fn replace_terrain_texture_layer(
        &mut self,
        layer_index: u32,
        albedo_data: &[u8],
        normal_data: &[u8],
        mra_data: &[u8],
    ) {
        // Ensure terrain renderer exists before attempting texture replacement
        if self.terrain_renderer.is_none() {
            eprintln!(
                "=== replace_terrain_texture_layer: terrain_renderer was NONE — creating it now"
            );
            let _ = self.ensure_terrain_renderer();
        }
        if let Some(tr) = &self.terrain_renderer {
            eprintln!(
                "=== replace_terrain_texture_layer: writing layer {} (albedo={}B)",
                layer_index,
                albedo_data.len()
            );
            tr.replace_texture_layer(layer_index, albedo_data, normal_data, mra_data);
        } else {
            eprintln!(
                "=== replace_terrain_texture_layer: FAILED — terrain_renderer still NONE after ensure!"
            );
        }
    }

    /// Ensure a scatter mesh is loaded and cached.
    pub fn ensure_scatter_mesh(&mut self, key: &str, path: &str) -> Result<()> {
        self.ensure_scatter_renderer()?
            .ensure_mesh_loaded(key, path)
    }

    /// Set wind parameters for scatter vegetation animation.
    pub fn set_scatter_wind(&mut self, strength: f32, frequency: f32) {
        if let Ok(scatter) = self.ensure_scatter_renderer() {
            scatter.set_wind(strength, frequency);
        }
    }

    /// Set cull distance for scatter objects.
    pub fn set_scatter_cull_distance(&mut self, distance: f32) {
        if let Ok(scatter) = self.ensure_scatter_renderer() {
            scatter.set_cull_distance(distance);
        }
    }

    /// Get the number of scatter instances rendered last frame.
    pub fn scatter_instance_count(&self) -> u32 {
        self.scatter_renderer
            .as_ref()
            .map_or(0, |s| s.last_instance_count())
    }

    /// Get the number of scatter draw calls last frame.
    pub fn scatter_draw_calls(&self) -> u32 {
        self.scatter_renderer
            .as_ref()
            .map_or(0, |s| s.last_draw_calls())
    }

    /// Total triangles rendered by the terrain renderer.
    pub fn terrain_triangles(&self) -> usize {
        self.terrain_renderer
            .as_ref()
            .map_or(0, |t| t.total_triangles())
    }

    /// Total indices rendered by the terrain renderer.
    pub fn terrain_indices(&self) -> usize {
        self.terrain_renderer
            .as_ref()
            .map_or(0, |t| t.total_indices())
    }

    /// Total triangles rendered by the scatter renderer last frame.
    pub fn scatter_triangles(&self) -> usize {
        self.scatter_renderer
            .as_ref()
            .map_or(0, |s| s.last_total_triangles())
    }

    /// Total vertices rendered by the scatter renderer last frame.
    pub fn scatter_vertices(&self) -> usize {
        self.scatter_renderer
            .as_ref()
            .map_or(0, |s| s.last_total_vertices())
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
