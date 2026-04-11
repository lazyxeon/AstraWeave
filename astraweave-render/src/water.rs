//! Water rendering system with animated Gerstner waves
//!
//! Provides realistic ocean simulation with:
//! - 4 summed Gerstner wave components
//! - Fresnel-based reflections
//! - Depth-based color blending
//! - Animated foam on wave crests

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// Water uniforms for shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterUniforms {
    pub view_proj: [[f32; 4]; 4],      // 0-64
    pub camera_pos: [f32; 3],          // 64-76
    pub time: f32,                     // 76-80
    pub water_color_deep: [f32; 3],    // 80-92
    pub _pad0: f32,                    // 92-96
    pub water_color_shallow: [f32; 3], // 96-108
    pub _pad1: f32,                    // 108-112
    pub foam_color: [f32; 3],          // 112-124
    pub foam_threshold: f32,           // 124-128
    pub rain_intensity: f32,           // 128-132
    pub ripple_scale: f32,             // 132-136
    pub ripple_strength: f32,          // 136-140
    pub _pad2: f32,                    // 140-144
}

impl Default for WaterUniforms {
    fn default() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0.0, 5.0, -10.0],
            time: 0.0,
            water_color_deep: [0.02, 0.08, 0.2], // Deep ocean blue
            _pad0: 0.0,
            water_color_shallow: [0.1, 0.4, 0.5], // Turquoise shallow
            _pad1: 0.0,
            foam_color: [0.95, 0.98, 1.0], // White foam
            foam_threshold: 0.6,
            rain_intensity: 0.0,
            ripple_scale: 4.0,
            ripple_strength: 0.15,
            _pad2: 0.0,
        }
    }
}

/// Water vertex (position + UV)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WaterVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl WaterVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<WaterVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

/// Water rendering system
pub struct WaterRenderer {
    pipeline: wgpu::RenderPipeline,
    _bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    uniforms: WaterUniforms,
}

impl WaterRenderer {
    /// Create a new water renderer
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        // Load shader
        let shader_source = include_str!("shaders/water.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water_shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // Uniform buffer
        let uniforms = WaterUniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water_uniforms"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("water_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Render pipeline with alpha blending
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[WaterVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // DEBUG: Render both sides
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: false, // Transparent, don't write depth
                depth_compare: wgpu::CompareFunction::LessEqual, // Normal depth testing
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Generate water plane mesh
        // Generate water plane mesh (larger 500x500 area, 128x128 grid)
        let (vertices, indices) = Self::generate_water_plane(500.0, 128);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            pipeline,
            _bind_group_layout: bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            uniforms,
        }
    }

    /// Generate a subdivided water plane
    fn generate_water_plane(size: f32, subdivisions: u32) -> (Vec<WaterVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        let half_size = size / 2.0;
        let step = size / subdivisions as f32;

        // Generate vertices
        for z in 0..=subdivisions {
            for x in 0..=subdivisions {
                let pos_x = -half_size + x as f32 * step;
                let pos_z = -half_size + z as f32 * step;
                let u = x as f32 / subdivisions as f32;
                let v = z as f32 / subdivisions as f32;

                vertices.push(WaterVertex {
                    position: [pos_x, 2.0, pos_z], // Normal Water Level (Y=2.0)

                    uv: [u, v],
                });
            }
        }

        // Generate indices
        for z in 0..subdivisions {
            for x in 0..subdivisions {
                let top_left = z * (subdivisions + 1) + x;
                let top_right = top_left + 1;
                let bottom_left = (z + 1) * (subdivisions + 1) + x;
                let bottom_right = bottom_left + 1;

                // First triangle
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // Second triangle
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        (vertices, indices)
    }

    /// Update water state for animation
    pub fn update(&mut self, queue: &wgpu::Queue, view_proj: Mat4, camera_pos: Vec3, time: f32) {
        self.uniforms.view_proj = view_proj.to_cols_array_2d();
        self.uniforms.camera_pos = camera_pos.into();
        self.uniforms.time = time;
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));
    }

    /// Set water level (Y position)
    pub fn set_water_level(&mut self, _level: f32) {
        // Water level is controlled by the uniform, already at y=0 in mesh
    }

    /// Set biome-driven water colors (deep, shallow, foam).
    ///
    /// Colours are applied on the next [`Self::update`] call which
    /// uploads the uniform buffer to the GPU.
    pub fn set_water_colors(&mut self, deep: Vec3, shallow: Vec3, foam: Vec3) {
        self.uniforms.water_color_deep = deep.into();
        self.uniforms.water_color_shallow = shallow.into();
        self.uniforms.foam_color = foam.into();
    }

    /// Set rain intensity for ripple effects on the water surface.
    ///
    /// `intensity`: 0.0 = no rain, 1.0 = heavy rain.
    /// Applied on the next [`Self::update`] call.
    pub fn set_rain_intensity(&mut self, intensity: f32) {
        self.uniforms.rain_intensity = intensity.clamp(0.0, 1.0);
    }

    /// Get current water colors (deep, shallow, foam).
    pub fn water_colors(&self) -> (Vec3, Vec3, Vec3) {
        (
            Vec3::from(self.uniforms.water_color_deep),
            Vec3::from(self.uniforms.water_color_shallow),
            Vec3::from(self.uniforms.foam_color),
        )
    }

    /// Render the water surface
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_water_plane_generation() {
        let (vertices, indices) = WaterRenderer::generate_water_plane(10.0, 4);
        assert_eq!(vertices.len(), 25); // (4+1)^2
        assert_eq!(indices.len(), 96); // 4*4*6
    }

    #[test]
    fn test_uniforms_size() {
        // Ensure uniform struct is properly aligned for GPU
        assert_eq!(std::mem::size_of::<WaterUniforms>(), 144);
    }

    #[test]
    fn test_water_vertex_desc() {
        let desc = WaterVertex::desc();
        assert_eq!(desc.array_stride, std::mem::size_of::<WaterVertex>() as u64);
        assert_eq!(desc.attributes.len(), 2);
    }

    #[test]
    fn test_water_renderer_new_and_update() {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await;
            if let Ok(adapter) = adapter {
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor::default())
                    .await
                    .unwrap();
                let mut renderer = WaterRenderer::new(
                    &device,
                    wgpu::TextureFormat::Rgba8UnormSrgb,
                    wgpu::TextureFormat::Depth32Float,
                );

                assert_eq!(renderer.index_count, 128 * 128 * 6);

                let view_proj = Mat4::IDENTITY;
                let camera_pos = Vec3::new(1.0, 2.0, 3.0);
                let time = 10.0;

                renderer.update(&queue, view_proj, camera_pos, time);

                assert_eq!(renderer.uniforms.time, 10.0);
                assert_eq!(renderer.uniforms.camera_pos, [1.0, 2.0, 3.0]);

                renderer.set_water_level(5.0);
            }
        });
    }
}
