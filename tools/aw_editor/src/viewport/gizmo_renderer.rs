//! Gizmo Renderer (wgpu integration)
//!
//! Renders translate/rotate/scale gizmos using wgpu line rendering.
//! Integrates with existing gizmo module for geometry generation.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use glam::{Quat, Vec3};
use tracing::debug;

use super::camera::OrbitCamera;
use crate::gizmo::{
    AxisConstraint, GizmoMode, GizmoRenderParams, GizmoRenderer as GizmoGeometry, GizmoState,
};

/// Gizmo renderer for viewport
///
/// Renders 3D gizmos (translate arrows, rotate circles, scale cubes)
/// on top of selected entities using line rendering.
pub struct GizmoRendererWgpu {
    /// Line rendering pipeline
    pipeline: wgpu::RenderPipeline,

    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group (camera uniforms)
    bind_group: wgpu::BindGroup,

    /// Camera uniform buffer
    uniform_buffer: wgpu::Buffer,

    /// Vertex buffer (dynamic, updated per frame)
    vertex_buffer: wgpu::Buffer,

    /// Maximum vertices per frame
    max_vertices: usize,

    /// wgpu device reference (for dynamic buffer updates)
    device: wgpu::Device,

    /// wgpu queue reference
    queue: wgpu::Queue,
}

impl GizmoRendererWgpu {
    /// Create new gizmo renderer
    ///
    /// # Arguments
    ///
    /// * `device` - wgpu device
    /// * `queue` - wgpu queue
    /// * `max_vertices` - Maximum vertices per frame (default: 10000)
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, max_vertices: usize) -> Result<Self> {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Gizmo Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gizmo.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Gizmo Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Gizmo Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipeline (line rendering)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Gizmo Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GizmoVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3, // position
                        },
                        wgpu::VertexAttribute {
                            offset: 12,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3, // color
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for lines
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Don't write depth (always on top)
                depth_compare: wgpu::CompareFunction::Always, // Always render
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gizmo Uniform Buffer"),
            size: std::mem::size_of::<GizmoUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Gizmo Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create vertex buffer (dynamic)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Gizmo Vertex Buffer"),
            size: (std::mem::size_of::<GizmoVertex>() * max_vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            max_vertices,
            device,
            queue,
        })
    }

    /// Render gizmo for selected entity
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder
    /// * `target` - Render target view
    /// * `depth` - Depth buffer view
    /// * `camera` - Camera for view-projection
    /// * `gizmo_state` - Current gizmo state (mode, constraint, etc.)
    /// * `entity_position` - 2D grid position of selected entity
    /// * `hovered_axis` - Currently hovered axis for visual highlighting
    /// * `queue` - wgpu queue
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
        gizmo_state: &GizmoState,
        entity_position: glam::IVec2,
        entity_height: f32,
        hovered_axis: Option<AxisConstraint>,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        // Early exit if no active gizmo
        if gizmo_state.mode == GizmoMode::Inactive {
            return Ok(());
        }

        // Convert 2D grid position to 3D world position
        let world_position = Vec3::new(
            entity_position.x as f32,
            entity_height,
            entity_position.y as f32,
        );
        let world_rotation = Quat::IDENTITY; // No rotation (top-down 2D game)

        // Update camera uniforms — camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let cam_pos = camera.position();
        let uniforms = GizmoUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let camera_distance = (camera.position() - world_position).length();
        let gizmo_scale = (camera_distance * 0.08).clamp(0.1, 10.0);

        // Extract constraint from mode
        let constraint = match gizmo_state.mode {
            GizmoMode::Translate { constraint } => constraint,
            GizmoMode::Rotate { constraint } => constraint,
            GizmoMode::Scale { constraint, .. } => constraint,
            GizmoMode::Inactive => AxisConstraint::None,
        };

        // DEBUG: Log constraint for rotate mode
        if matches!(gizmo_state.mode, GizmoMode::Rotate { .. }) {
            debug!("Gizmo Renderer: Rotate constraint = {:?}", constraint);
        }

        let params = GizmoRenderParams {
            position: world_position,
            rotation: world_rotation,
            scale: gizmo_scale,
            camera_pos: camera.position(),
            view_proj,
            mode: gizmo_state.mode,
            constraint,
            hovered_axis, // Pass through for visual highlighting
        };

        let geometries = match gizmo_state.mode {
            GizmoMode::Translate { .. } => GizmoGeometry::render_translation(&params),
            GizmoMode::Rotate { .. } => GizmoGeometry::render_rotation(&params),
            GizmoMode::Scale { .. } => GizmoGeometry::render_scale(&params),
            GizmoMode::Inactive => return Ok(()),
        };

        // Convert geometries to vertices (world space)
        let mut vertices = self.geometries_to_vertices(&geometries, world_position, world_rotation);

        // Offset to camera-relative coordinates (VP matrix has eye at origin)
        for v in &mut vertices {
            v.position[0] -= cam_pos.x;
            v.position[1] -= cam_pos.y;
            v.position[2] -= cam_pos.z;
        }

        if vertices.is_empty() {
            return Ok(());
        }

        let vertex_count = vertices.len().min(self.max_vertices);

        // Write vertices to buffer
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices[..vertex_count]),
        );

        // Render pass
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Gizmo Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Don't clear (render on top)
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Use existing depth
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..vertex_count as u32, 0..1);

        Ok(())
    }

    /// Convert gizmo geometries to line vertices
    ///
    /// Takes geometry tuples (vertices, color, highlighted) and converts
    /// to line segments for rendering.
    fn geometries_to_vertices(
        &self,
        geometries: &[(Vec<Vec3>, [f32; 3], bool)],
        position: Vec3,
        rotation: Quat,
    ) -> Vec<GizmoVertex> {
        let mut vertices = Vec::new();

        for (geom_vertices, color, _highlighted) in geometries {
            // Transform vertices to world space
            let world_vertices: Vec<Vec3> = geom_vertices
                .iter()
                .map(|&v| position + rotation * v)
                .collect();

            // Convert to line segments (consecutive pairs)
            for i in 0..world_vertices.len().saturating_sub(1) {
                vertices.push(GizmoVertex {
                    position: world_vertices[i].to_array(),
                    color: *color,
                });
                vertices.push(GizmoVertex {
                    position: world_vertices[i + 1].to_array(),
                    color: *color,
                });
            }
        }

        vertices
    }
}

/// Gizmo vertex (position + color)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GizmoVertex {
    position: [f32; 3],
    color: [f32; 3],
}

/// Gizmo shader uniforms
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GizmoUniforms {
    view_proj: [[f32; 4]; 4],
}
