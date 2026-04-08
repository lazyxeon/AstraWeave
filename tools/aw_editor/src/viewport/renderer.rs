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
use wgpu::util::DeviceExt;

/// Color format used for the HDR scene render target.
/// All scene sub-renderers create their pipelines against this format.
const HDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// Color format used for the final LDR output (egui display surface).
const LDR_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

use super::camera::OrbitCamera;
use super::gizmo_renderer::GizmoRendererWgpu;
use super::grid_renderer::GridRenderer;
use super::physics_renderer::PhysicsDebugRenderer;
use super::types::{
    GltfAnimationClip, GltfSkeleton, ScatterPlacement, SceneLight, TerrainFogParams,
    TerrainLightingParams, TerrainVertex, WaterStyle,
};
use crate::gizmo::GizmoState;
use astraweave_core::{Entity, World};

use super::engine_adapter::EngineRenderAdapter;
pub use super::engine_adapter::RenderMode;

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
    gizmo_renderer: GizmoRendererWgpu,

    /// Sub-renderers (deferred — created lazily on first use)
    physics_renderer: Option<PhysicsDebugRenderer>,

    /// Scatter placements forwarded to engine adapter each frame
    scatter_placements: Vec<ScatterPlacement>,

    /// Engine renderer adapter for PBR mesh rendering
    engine_adapter: Option<EngineRenderAdapter>,

    /// Enable engine rendering (PBR meshes) vs cube rendering
    render_mode: RenderMode,

    /// HDR scene render target (Rgba16Float) — all scene passes render here
    hdr_texture: Option<wgpu::Texture>,

    /// HDR scene render target view
    hdr_view: Option<wgpu::TextureView>,

    /// Tonemap pipeline (HDR → LDR blit with ACES tonemapping)
    tonemap_pipeline: Option<wgpu::RenderPipeline>,

    /// Tonemap bind group layout
    tonemap_bind_group_layout: Option<wgpu::BindGroupLayout>,

    /// Tonemap bind group (references HDR texture + params uniform)
    tonemap_bind_group: Option<wgpu::BindGroup>,

    /// Tonemap params uniform buffer (mode selection)
    tonemap_params_buffer: Option<wgpu::Buffer>,

    /// Active tonemapper: 0=ACES, 1=PBR Neutral, 2=Reinhard
    tonemap_mode: u32,

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

    /// Entity-to-mesh path mapping (for engine adapter entity feeding)
    entity_mesh_map: std::collections::HashMap<Entity, String>,

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
        // Only create essential renderers at startup (grid, gizmos).
        // Physics debug renderer is deferred to first use.
        // Entity rendering is handled by the engine adapter (PBR path).
        let grid_renderer = GridRenderer::with_color_format(&device, HDR_COLOR_FORMAT)
            .context("Failed to create grid renderer")?;
        let gizmo_renderer = GizmoRendererWgpu::new((*device).clone(), (*queue).clone(), 10000)
            .context("Failed to create gizmo renderer")?;

        Ok(Self {
            device,
            queue,
            grid_renderer,
            gizmo_renderer,
            physics_renderer: None,
            scatter_placements: Vec::new(),
            engine_adapter: None,
            render_mode: RenderMode::EnginePBR,
            hdr_texture: None,
            hdr_view: None,
            tonemap_pipeline: None,
            tonemap_bind_group_layout: None,
            tonemap_bind_group: None,
            tonemap_params_buffer: None,
            tonemap_mode: 0, // Default: ACES
            depth_texture: None,
            depth_view: None,
            size: (0, 0),
            depth_staging_buffer: None,
            depth_read_pending: false,
            depth_map_ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            cached_depth_value: None,
            selected_entities: Vec::new(),
            entity_mesh_map: std::collections::HashMap::new(),
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

    fn ensure_physics_renderer(&mut self) -> Result<&mut PhysicsDebugRenderer> {
        if self.physics_renderer.is_none() {
            self.physics_renderer = Some(
                PhysicsDebugRenderer::with_color_format(
                    (*self.device).clone(),
                    (*self.queue).clone(),
                    5000,
                    HDR_COLOR_FORMAT,
                )
                .context("Failed to create physics debug renderer (deferred)")?,
            );
        }
        self.physics_renderer
            .as_mut()
            .context("physics renderer not initialized after creation")
    }

    /// Eagerly initialize deferred renderers to avoid frame hitches during gameplay.
    /// Call once after the first frame has rendered (GPU device is warm).
    pub fn eagerly_init_all(&mut self) {
        if self.physics_renderer.is_none() {
            match PhysicsDebugRenderer::with_color_format(
                (*self.device).clone(),
                (*self.queue).clone(),
                5000,
                HDR_COLOR_FORMAT,
            ) {
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
            // Invalid size, clear render targets
            self.depth_texture = None;
            self.depth_view = None;
            self.hdr_texture = None;
            self.hdr_view = None;
            self.tonemap_bind_group = None;
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

        // Create HDR scene render target (Rgba16Float)
        let hdr_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Viewport HDR Scene Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let hdr_view = hdr_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create or recreate the tonemap pipeline and bind group
        self.create_tonemap_resources(&hdr_view);

        self.hdr_texture = Some(hdr_texture);
        self.hdr_view = Some(hdr_view);

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
        _shading_mode: u32,
    ) -> Result<()> {
        astraweave_profiling::span!("viewport_render");
        // Ensure depth buffer matches target size
        let target_size = target.size();
        if self.size != (target_size.width, target_size.height) {
            self.resize(target_size.width, target_size.height)
                .context("Failed to resize depth buffer")?;
        }

        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        // Eagerly init physics renderer if component gizmo lines, physics lines, brush cursor, or zone overlay exist
        if (!self.component_gizmo_lines.is_empty()
            || physics_debug_lines.is_some()
            || !self.brush_cursor_lines.is_empty()
            || !self.zone_overlay_lines.is_empty())
            && self.physics_renderer.is_none()
        {
            let _ = self.ensure_physics_renderer();
        }

        // All scene passes render to the HDR target (Rgba16Float).
        // After scene rendering + post-processing, a tonemap blit converts
        // HDR → LDR (Bgra8UnormSrgb) for the final display surface.
        let scene_target_view = self
            .hdr_view
            .as_ref()
            .context("HDR render target not initialized — call resize() first")?;

        let depth_view = self
            .depth_view
            .as_ref()
            .context("Depth buffer not initialized")?;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Viewport Render Encoder"),
            });

        // ── Unified engine render path ────────────────────────────────────
        // The engine renderer handles: sky, shadows, terrain, scatter,
        // water, entities, weather particles, post-processing.
        // Editor overlays (grid, gizmo, physics debug) render on top.
        {
            astraweave_profiling::span!("engine_render");
            if let Some(adapter) = self.engine_adapter.as_mut() {
                adapter.update_camera(camera);
                adapter.feed_entities(world, &self.entity_mesh_map, &self.selected_entities);
                adapter
                    .render_to_texture(scene_target_view, &mut encoder)
                    .context("Engine render failed")?;
            } else {
                // Headless/fallback: clear to dark background when engine adapter unavailable
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Headless Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: scene_target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.12,
                                g: 0.12,
                                b: 0.15,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
        } // end engine_render span

        // Grid overlay (on top of engine scene)
        if show_grid {
            astraweave_profiling::span!("grid_render");
            self.grid_renderer
                .render(
                    &mut encoder,
                    scene_target_view,
                    depth_view,
                    camera,
                    &self.queue,
                    crosshair_mode,
                )
                .context("Grid render failed")?;
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
                            scene_target_view,
                            depth_view,
                            camera,
                            &combined_lines,
                        )
                        .context("Physics debug render failed")?;
                }
            }
        }

        // Pass 5.0: Blit HDR → LDR
        // The engine's draw_into() writes tonemapped output to the HDR buffer.
        // This pass copies it to the final LDR display surface (Bgra8UnormSrgb).
        if let (Some(pipeline), Some(bind_group)) =
            (&self.tonemap_pipeline, &self.tonemap_bind_group)
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("HDR to LDR Blit Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Pass 5.5: Gizmos (if entity selected and gizmo active)
        // Gizmos render AFTER post-processing onto the final LDR target for crisp overlays.
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

    /// Create the tonemap pipeline and bind group for HDR → LDR blit.
    /// Called from `resize()` whenever the HDR target is (re)created.
    fn create_tonemap_resources(&mut self, hdr_view: &wgpu::TextureView) {
        let device = &self.device;

        // Bind group layout: HDR texture + sampler + tonemap params
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tonemap Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Sampler (bilinear, clamp)
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Tonemap HDR Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Tonemap params uniform buffer
        let params_data: [u32; 4] = [self.tonemap_mode, 0, 0, 0];
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tonemap Params Buffer"),
            contents: bytemuck::cast_slice(&params_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tonemap Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });
        self.tonemap_params_buffer = Some(params_buffer);

        // Pipeline (only create once — the layout doesn't change)
        if self.tonemap_pipeline.is_none() {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Tonemap Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tonemap.wgsl").into()),
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Tonemap Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Tonemap Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: LDR_COLOR_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

            self.tonemap_pipeline = Some(pipeline);
        }

        self.tonemap_bind_group_layout = Some(bind_group_layout);
        self.tonemap_bind_group = Some(bind_group);
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

    /// Set the entity-to-mesh mapping so models render with actual GLTF geometry.
    /// Populates `entity_mesh_map` which `feed_entities()` uses to pass data to the
    /// engine adapter.
    pub fn set_entity_meshes(&mut self, meshes: std::collections::HashMap<Entity, String>) {
        self.entity_mesh_map = meshes;
    }

    /// Set per-entity external texture overrides.
    /// No-op: texture overrides are handled by the engine PBR material pipeline.
    pub fn set_entity_texture_overrides(
        &mut self,
        _overrides: std::collections::HashMap<Entity, String>,
    ) {
        // Legacy entity renderer handled per-entity texture overrides.
        // Engine adapter manages materials through its own PBR pipeline.
    }

    /// Get skeleton for a mesh.
    /// Returns `None` — skeleton data is no longer stored in the legacy renderer.
    /// Future: engine adapter will expose skeleton data from its own mesh cache.
    pub fn get_mesh_skeleton(&self, _mesh_path: &str) -> Option<&GltfSkeleton> {
        None
    }

    /// Get animation clips for a mesh.
    /// Returns empty — animation clips are no longer stored in the legacy renderer.
    /// Future: engine adapter will expose animation data from its own mesh cache.
    pub fn get_mesh_animations(&self, _mesh_path: &str) -> &[GltfAnimationClip] {
        &[]
    }

    /// Apply CPU skinning to a mesh.
    /// No-op: skinning will be handled by the engine's GPU skinning pipeline.
    pub fn apply_cpu_skinning(&mut self, _mesh_path: &str, _joint_matrices: &[glam::Mat4]) {
        // Legacy CPU skinning removed. Engine adapter handles GPU skinning.
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
    /// Set the active tonemapper: 0=ACES, 1=PBR Neutral, 2=Reinhard
    pub fn set_tonemap_mode(&mut self, mode: u32) {
        self.tonemap_mode = mode.min(2);
        // Update the GPU buffer if it exists
        if let Some(buffer) = &self.tonemap_params_buffer {
            let params_data: [u32; 4] = [self.tonemap_mode, 0, 0, 0];
            self.queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&params_data));
        }
    }

    /// Get the current tonemapper mode: 0=ACES, 1=PBR Neutral, 2=Reinhard
    pub fn tonemap_mode(&self) -> u32 {
        self.tonemap_mode
    }

    /// Set the editor quality preset (shadows + post-processing).
    pub fn set_quality_preset(&mut self, preset: super::engine_adapter::EditorQualityPreset) {
        if let Some(adapter) = self.engine_adapter.as_mut() {
            adapter.apply_quality_preset(preset);
        }
    }

    /// Get the current editor quality preset.
    pub fn quality_preset(&self) -> super::engine_adapter::EditorQualityPreset {
        self.engine_adapter
            .as_ref()
            .map(|a| a.quality_preset())
            .unwrap_or_default()
    }

    /// Get GPU memory usage: (used_bytes, budget_bytes, percentage).
    pub fn gpu_memory_stats(&self) -> (u64, u64, f32) {
        self.engine_adapter
            .as_ref()
            .map(|a| a.gpu_memory_stats())
            .unwrap_or((0, 0, 0.0))
    }

    pub fn handle_device_lost(&mut self) -> Result<()> {
        tracing::error!("GPU device lost in ViewportRenderer - cleaning up resources for recovery");

        // Clear resources that depend on the old device
        self.depth_texture = None;
        self.depth_view = None;

        self.engine_adapter = None;

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

    pub fn upload_terrain_chunks(&mut self, chunks: &[(Vec<TerrainVertex>, Vec<u32>)]) {
        // Forward to engine adapter
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.upload_terrain_chunks(chunks);
        } else {
            tracing::warn!(
                "upload_terrain_chunks: engine adapter not initialized — {} chunks dropped",
                chunks.len()
            );
        }
    }

    /// Upload terrain chunks using the terrain_integration vertex type directly
    /// (zero-copy path — avoids redundant field-by-field vertex remapping).
    pub fn upload_terrain_chunks_raw(
        &mut self,
        chunks: &[(Vec<crate::terrain_integration::TerrainVertex>, Vec<u32>)],
    ) {
        // Auto-initialize the engine adapter if it hasn't been created yet.
        // The adapter is normally lazy-initialized on first glTF load, but
        // terrain generation can happen before any model is loaded.
        if self.engine_adapter.is_none() {
            use pollster::FutureExt;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.init_engine_adapter().block_on()
            }));
            match result {
                Ok(Ok(())) => {
                    tracing::info!("Engine adapter auto-initialized for terrain rendering");
                    // Load default grassland terrain texture
                    self.load_default_terrain_texture();
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to initialize engine adapter for terrain: {e}");
                    return;
                }
                Err(_) => {
                    tracing::error!(
                        "Engine adapter initialization panicked — terrain upload skipped"
                    );
                    return;
                }
            }
        }

        // Re-map terrain_integration::TerrainVertex → viewport TerrainVertex for engine adapter.
        if let Some(adapter) = &mut self.engine_adapter {
            let converted: Vec<(Vec<TerrainVertex>, Vec<u32>)> = chunks
                .iter()
                .map(|(verts, indices)| {
                    let mapped: Vec<TerrainVertex> = verts
                        .iter()
                        .map(|v| TerrainVertex {
                            position: v.position,
                            normal: v.normal,
                            uv: v.uv,
                            biome_weights_0: v.biome_weights_0,
                            biome_weights_1: v.biome_weights_1,
                            material_ids: v.material_ids,
                            material_weights: v.material_weights,
                        })
                        .collect();
                    (mapped, indices.clone())
                })
                .collect();
            adapter.upload_terrain_chunks(&converted);
        } else {
            tracing::warn!(
                "upload_terrain_chunks_raw: engine adapter not initialized — {} chunks dropped",
                chunks.len()
            );
        }
    }

    /// Incrementally update vertex data for a single terrain chunk on the GPU.
    ///
    /// Delegates to the engine adapter's `update_terrain_chunk()` which replaces
    /// the chunk model with fresh vertex data while reusing cached indices.
    pub fn update_terrain_chunk_vertices(
        &mut self,
        chunk_index: usize,
        vertices: &[TerrainVertex],
    ) {
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.update_terrain_chunk(chunk_index, vertices);
        } else {
            tracing::warn!("update_terrain_chunk_vertices: engine adapter not initialized");
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
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.clear_terrain();
        }
    }

    pub fn terrain_chunk_count(&self) -> usize {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.terrain_chunk_count();
        }
        0
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
        if let Some(adapter) = &self.engine_adapter {
            return adapter.weather_active();
        }
        false
    }

    /// Load an HDRI file as the skybox background via the engine adapter.
    pub fn load_hdri(&mut self, path: &std::path::Path) -> Result<()> {
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.load_hdri(path)
        } else {
            Err(anyhow::anyhow!(
                "Engine adapter not initialized — cannot load HDRI"
            ))
        }
    }

    /// Remove the HDRI skybox and revert to procedural sky.
    pub fn clear_hdri(&mut self) {
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.renderer_mut().ibl_mut().mode = astraweave_render::ibl::SkyMode::Procedural {
                last_capture_time: 0.0,
                recapture_interval: 5.0,
            };
            if let Err(e) = adapter
                .renderer_mut()
                .bake_environment(astraweave_render::ibl::IblQuality::Medium)
            {
                tracing::warn!("Failed to rebake environment after HDRI clear: {e}");
            }
        }
    }

    /// Set environment sky colors (for skybox presets, time-of-day, weather)
    pub fn set_sky_colors(
        &mut self,
        sky_top: [f32; 4],
        sky_horizon: [f32; 4],
        _ground_color: [f32; 4],
    ) {
        // Forward to engine adapter
        if let Some(adapter) = &mut self.engine_adapter {
            let mut cfg = adapter.sky_config();
            cfg.day_color_top = glam::Vec3::new(sky_top[0], sky_top[1], sky_top[2]);
            cfg.day_color_horizon = glam::Vec3::new(sky_horizon[0], sky_horizon[1], sky_horizon[2]);
            adapter.set_sky_config(cfg);
        }
    }

    /// Set fog and weather parameters for distance-based terrain fog
    pub fn set_fog_params(&mut self, params: TerrainFogParams) {
        // Forward to engine adapter
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.set_fog_params(&params);
        }
    }

    /// Set lighting parameters for PBR terrain shading
    pub fn set_lighting_params(&mut self, params: TerrainLightingParams) {
        // Forward to engine adapter (handles all scene lighting)
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.set_lighting_params(&params);
        }
    }

    /// Set scene point lights from entity Light components.
    /// No-op: scene lights are now managed by the engine adapter's clustered lighting.
    pub fn set_scene_lights(&mut self, _lights: Vec<SceneLight>) {
        // Legacy entity renderer handled per-entity point lights.
        // Engine adapter manages lights through its own clustered lighting pipeline.
    }

    /// Set water level for volumetric water plane
    pub fn set_water_level(&mut self, _level: f32) {
        // Water is now handled by engine adapter.
    }

    /// Enable or disable the volumetric water plane
    pub fn set_water_enabled(&mut self, enabled: bool) {
        // Forward water state to engine adapter
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.set_water_enabled(enabled, WaterStyle::Ocean);
        }
    }

    // ── Scatter management ──────────────────────────────────────────────

    /// Set scatter placements for instanced rendering.
    /// Preload glTF meshes into the mesh cache.
    /// No-op stub: the engine adapter loads meshes on demand via its own asset pipeline.
    /// Returns 0 (no meshes loaded through the legacy path).
    pub fn preload_gltf_meshes(&mut self, _paths: &[String]) -> usize {
        0
    }

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

        // Forward scatter to engine adapter
        if let Some(adapter) = &mut self.engine_adapter {
            adapter.upload_scatter_placements(&placements);
        }

        self.scatter_placements = placements;
    }

    // ── Terrain texture management ────────────────────────────────────

    /// Replace a single material layer's textures.
    /// Terrain textures are managed by the engine adapter's material system.
    pub fn replace_terrain_texture_layer(
        &mut self,
        _layer_index: u32,
        _albedo_data: &[u8],
        _normal_data: &[u8],
        _mra_data: &[u8],
    ) {
        tracing::warn!(
            "replace_terrain_texture_layer: not supported in engine render mode — \
             terrain textures are managed by the engine adapter's PBR material pipeline"
        );
    }

    /// Ensure a scatter mesh is loaded and cached.
    /// No-op: scatter rendering is handled by the engine adapter.
    pub fn ensure_scatter_mesh(&mut self, _key: &str, _path: &str) -> Result<()> {
        Ok(())
    }

    /// Set wind parameters for scatter vegetation animation.
    pub fn set_scatter_wind(&mut self, _strength: f32, _frequency: f32) {}

    /// Set cull distance for scatter objects.
    pub fn set_scatter_cull_distance(&mut self, _distance: f32) {}

    /// Get the number of scatter instances rendered last frame.
    pub fn scatter_instance_count(&self) -> u32 {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.scatter_instance_count() as u32;
        }
        0
    }

    /// Get the number of scatter draw calls last frame.
    pub fn scatter_draw_calls(&self) -> u32 {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.scatter_draw_calls();
        }
        0
    }

    /// Total triangles rendered by the terrain renderer.
    pub fn terrain_triangles(&self) -> usize {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.terrain_triangles();
        }
        0
    }

    /// Total indices rendered by the terrain renderer.
    pub fn terrain_indices(&self) -> usize {
        if let Some(adapter) = &self.engine_adapter {
            return adapter.terrain_indices();
        }
        0
    }

    /// Total triangles rendered by the scatter renderer last frame.
    pub fn scatter_triangles(&self) -> usize {
        0
    }

    /// Total vertices rendered by the scatter renderer last frame.
    pub fn scatter_vertices(&self) -> usize {
        0
    }

    /// Check if engine rendering (PBR meshes) is enabled
    pub fn use_engine_rendering(&self) -> bool {
        self.render_mode == RenderMode::EnginePBR
    }

    /// Enable/disable engine rendering (PBR meshes vs cubes)
    pub fn set_use_engine_rendering(&mut self, enabled: bool) {
        self.render_mode = if enabled {
            RenderMode::EnginePBR
        } else {
            RenderMode::FastPreview
        };
    }

    /// Get the current render mode
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
    }

    /// Set render mode directly
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }

    /// Load a terrain albedo texture with a robust fallback chain.
    ///
    /// Tries multiple texture paths in order. If all fail, creates a 4x4 white
    /// fallback texture so terrain never renders black (the biome-tinted instance
    /// color still provides correct coloring).
    fn load_default_terrain_texture(&mut self) {
        self.load_biome_terrain_texture("Grassland");
    }

    /// Load the primary albedo texture for a specific biome.
    ///
    /// Each biome maps to its dominant ground material texture. If the
    /// biome-specific texture isn't found, falls back to the generic material
    /// PNG, then to a 4x4 white texture.
    pub fn load_biome_terrain_texture(&mut self, biome: &str) {
        // Per-biome texture candidates: first high-res, then material PNG fallback
        let candidates: Vec<&str> = match biome {
            "Desert" => vec![
                "assets/textures/aerial_beach_01_diff_4k.jpg",
                "assets/materials/sand.png",
            ],
            "Mountain" => vec![
                "assets/textures/aerial_rocks_01_diff_4k.jpg",
                "assets/materials/mountain_rock.png",
                "assets/materials/rock_slate.png",
            ],
            "Tundra" => vec!["assets/materials/snow.png", "assets/materials/ice.png"],
            "Forest" => vec![
                "assets/materials/forest_floor.png",
                "assets/materials/moss.png",
            ],
            "Swamp" => vec!["assets/materials/mud.png", "assets/materials/dirt.png"],
            "Beach" => vec![
                "assets/textures/aerial_beach_01_diff_4k.jpg",
                "assets/materials/sand.png",
            ],
            "River" => vec![
                "assets/materials/gravel.png",
                "assets/materials/cobblestone.png",
            ],
            "BiomePack" => vec![
                // Namaqualand and other decomposed blend biome packs
                "assets/imported/Namaqualand/textures/gravelly_sand_diff_4k.jpg.png",
                "assets/imported/Namaqualand/textures/damp_sand_diff.png.png",
                "assets/imported/Namaqualand/textures/cliff_side_diff_4k.jpg.png",
                "assets/textures/aerial_beach_01_diff_4k.jpg",
                "assets/materials/sand.png",
            ],
            _ => vec![
                // Grassland default
                "assets/textures/leafy_grass_diff_4k.jpg",
                "assets/materials/grass.png",
            ],
        };

        for path_str in &candidates {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                continue;
            }
            match image::open(path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width(), rgba.height());
                    if let Some(adapter) = &mut self.engine_adapter {
                        adapter.renderer_mut().set_albedo_from_rgba8(w, h, &rgba);
                        tracing::info!("Loaded {biome} terrain texture ({w}x{h}) from {path:?}");
                    }
                    return;
                }
                Err(e) => {
                    tracing::warn!("Failed to load terrain texture {path:?}: {e}");
                }
            }
        }

        // Ultimate fallback: 4x4 white texture
        tracing::warn!("No {biome} terrain texture found — using 4x4 white fallback");
        let white_data = vec![255u8; 4 * 4 * 4];
        if let Some(adapter) = &mut self.engine_adapter {
            adapter
                .renderer_mut()
                .set_albedo_from_rgba8(4, 4, &white_data);
        }
    }

    /// Initialize the engine renderer adapter (async, call once)
    ///
    /// Must be called before engine rendering can be used.
    /// Uses the viewport's current size for initialization.
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
    pub fn engine_adapter_initialized(&self) -> bool {
        self.engine_adapter.is_some()
    }

    /// Whether terrain chunks are currently loaded in the engine renderer.
    pub fn has_terrain(&self) -> bool {
        self.engine_adapter
            .as_ref()
            .is_some_and(|a| a.terrain_chunk_count() > 0)
    }

    /// Get immutable reference to engine adapter (if initialized)
    pub fn engine_adapter(&self) -> Option<&EngineRenderAdapter> {
        self.engine_adapter.as_ref()
    }

    /// Get mutable reference to engine adapter (if initialized)
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
        self.engine_adapter = None;
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
