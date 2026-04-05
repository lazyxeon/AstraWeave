//! Physics Debug Renderer
//!
//! Renders physics debug information including:
//! - Collider wireframes (from Rapier3D debug lines)
//! - Rigid body bounds
//! - Velocity vectors (optional)
//! - Contact points (optional)
//!
//! Uses the same line rendering approach as GizmoRenderer for consistency.

use anyhow::Result;
use bytemuck::{Pod, Zeroable};

use super::camera::OrbitCamera;

/// Physics debug visualization options
#[derive(Debug, Clone, Copy)]
pub struct PhysicsDebugOptions {
    /// Show collider wireframes
    pub show_colliders: bool,
    /// Show rigid body bounds
    pub show_bounds: bool,
    /// Show velocity vectors
    pub show_velocities: bool,
    /// Show contact points
    pub show_contacts: bool,
    /// Collider wireframe color
    pub collider_color: [f32; 3],
    /// Kinematic body color
    pub kinematic_color: [f32; 3],
    /// Static body color
    pub static_color: [f32; 3],
}

impl Default for PhysicsDebugOptions {
    fn default() -> Self {
        Self {
            show_colliders: true,
            show_bounds: false,
            show_velocities: false,
            show_contacts: false,
            collider_color: [0.0, 1.0, 0.5], // Cyan-green for dynamic
            kinematic_color: [1.0, 0.5, 0.0], // Orange for kinematic
            static_color: [0.5, 0.5, 0.5],   // Gray for static
        }
    }
}

/// Physics debug renderer
///
/// Renders physics debug lines (colliders, bounds, etc.) using line primitives.
/// Designed to be efficient for large numbers of colliders.
pub struct PhysicsDebugRenderer {
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
    /// wgpu queue reference
    queue: wgpu::Queue,
    /// Debug visualization options
    pub options: PhysicsDebugOptions,
}

impl PhysicsDebugRenderer {
    /// Create new physics debug renderer
    ///
    /// # Arguments
    ///
    /// * `device` - wgpu device
    /// * `queue` - wgpu queue
    /// * `max_vertices` - Maximum vertices per frame (default: 50000 for large scenes)
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, max_vertices: usize) -> Result<Self> {
        Self::with_color_format(
            device,
            queue,
            max_vertices,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        )
    }

    /// Create physics debug renderer with a configurable color target format.
    pub fn with_color_format(
        device: wgpu::Device,
        queue: wgpu::Queue,
        max_vertices: usize,
        color_format: wgpu::TextureFormat,
    ) -> Result<Self> {
        // Load shader (reuse gizmo shader - same vertex format)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Physics Debug Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/gizmo.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Physics Debug Bind Group Layout"),
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
            label: Some("Physics Debug Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipeline (line rendering)
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Physics Debug Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<PhysicsDebugVertex>() as u64,
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
                depth_write_enabled: false, // Don't write depth (render on top of scene)
                depth_compare: wgpu::CompareFunction::LessEqual, // Render with depth test
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
                    format: color_format,
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
            label: Some("Physics Debug Uniform Buffer"),
            size: std::mem::size_of::<PhysicsDebugUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Physics Debug Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create vertex buffer (dynamic)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Physics Debug Vertex Buffer"),
            size: (std::mem::size_of::<PhysicsDebugVertex>() * max_vertices) as u64,
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
            queue,
            options: PhysicsDebugOptions::default(),
        })
    }

    /// Render physics debug lines
    ///
    /// # Arguments
    ///
    /// * `encoder` - Command encoder
    /// * `target` - Render target view
    /// * `depth` - Depth buffer view
    /// * `camera` - Camera for view-projection
    /// * `debug_lines` - Physics debug lines from PhysicsWorld::get_debug_lines()
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
        debug_lines: &[astraweave_physics::DebugLine],
    ) -> Result<()> {
        // Early exit if no lines or disabled
        if debug_lines.is_empty() || !self.options.show_colliders {
            return Ok(());
        }

        // Update camera uniforms — camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let cam_pos = camera.position();
        let uniforms = PhysicsDebugUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Convert debug lines to vertices
        let mut vertices: Vec<PhysicsDebugVertex> = Vec::with_capacity(debug_lines.len() * 2);

        for line in debug_lines {
            // Offset positions to camera-relative coordinates (VP matrix has eye at origin)
            vertices.push(PhysicsDebugVertex {
                position: [
                    line.start[0] - cam_pos.x,
                    line.start[1] - cam_pos.y,
                    line.start[2] - cam_pos.z,
                ],
                color: line.color,
            });
            vertices.push(PhysicsDebugVertex {
                position: [
                    line.end[0] - cam_pos.x,
                    line.end[1] - cam_pos.y,
                    line.end[2] - cam_pos.z,
                ],
                color: line.color,
            });
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
            label: Some("Physics Debug Render Pass"),
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
                    load: wgpu::LoadOp::Load,
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
}

/// Physics debug vertex (position + color)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PhysicsDebugVertex {
    position: [f32; 3],
    color: [f32; 3],
}

/// Physics debug shader uniforms
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PhysicsDebugUniforms {
    view_proj: [[f32; 4]; 4],
}
