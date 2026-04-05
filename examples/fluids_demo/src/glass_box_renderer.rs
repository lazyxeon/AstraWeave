use glam::Mat4;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct GlassBoxRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GlassBoxRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        // Create cube mesh (120.0 x 60.0 x 60.0 tank)
        let (vertices, indices) = Self::create_cube_mesh(120.0, 60.0, 60.0);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Glass Box Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Glass Box Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Glass Box Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Glass Box Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Glass Box Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("glass.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Glass Box Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Glass Box Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
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
                cull_mode: None, // No culling for transparency
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Don't write depth for transparency
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
            uniform_buffer,
            bind_group_layout,
        }
    }

    fn create_cube_mesh(width: f32, height: f32, depth: f32) -> (Vec<Vertex>, Vec<u32>) {
        let hw = width / 2.0;
        let h = height;
        let hd = depth / 2.0;

        let vertices = vec![
            // Front face (Z+)
            Vertex {
                position: [-hw, 0.0, hd],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [hw, 0.0, hd],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [hw, h, hd],
                normal: [0.0, 0.0, 1.0],
            },
            Vertex {
                position: [-hw, h, hd],
                normal: [0.0, 0.0, 1.0],
            },
            // Back face (Z-)
            Vertex {
                position: [hw, 0.0, -hd],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [-hw, 0.0, -hd],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [-hw, h, -hd],
                normal: [0.0, 0.0, -1.0],
            },
            Vertex {
                position: [hw, h, -hd],
                normal: [0.0, 0.0, -1.0],
            },
            // Right face (X+)
            Vertex {
                position: [hw, 0.0, hd],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [hw, 0.0, -hd],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [hw, h, -hd],
                normal: [1.0, 0.0, 0.0],
            },
            Vertex {
                position: [hw, h, hd],
                normal: [1.0, 0.0, 0.0],
            },
            // Left face (X-)
            Vertex {
                position: [-hw, 0.0, -hd],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-hw, 0.0, hd],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-hw, h, hd],
                normal: [-1.0, 0.0, 0.0],
            },
            Vertex {
                position: [-hw, h, -hd],
                normal: [-1.0, 0.0, 0.0],
            },
            // Top face (Y+)
            Vertex {
                position: [-hw, h, hd],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [hw, h, hd],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [hw, h, -hd],
                normal: [0.0, 1.0, 0.0],
            },
            Vertex {
                position: [-hw, h, -hd],
                normal: [0.0, 1.0, 0.0],
            },
            // Bottom face (Y-)
            Vertex {
                position: [-hw, 0.0, -hd],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [hw, 0.0, -hd],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [hw, 0.0, hd],
                normal: [0.0, -1.0, 0.0],
            },
            Vertex {
                position: [-hw, 0.0, hd],
                normal: [0.0, -1.0, 0.0],
            },
        ];

        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0, // Back
            4, 5, 6, 6, 7, 4, // Right
            8, 9, 10, 10, 11, 8, // Left
            12, 13, 14, 14, 15, 12, // Top
            16, 17, 18, 18, 19, 16, // Bottom
            20, 21, 22, 22, 23, 20,
        ];

        (vertices, indices)
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        skybox_view: &wgpu::TextureView,
    ) {
        // Update uniforms
        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Glass Box Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(skybox_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Glass Box Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}
