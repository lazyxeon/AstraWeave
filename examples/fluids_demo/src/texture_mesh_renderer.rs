use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub transform: Mat4,
}

pub struct TextureMeshRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    textures: Vec<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup)>,
    meshes: Vec<(usize, wgpu::Buffer, wgpu::Buffer, u32, Mat4)>, // (texture_idx, vertex_buffer, index_buffer, index_count, transform)
}

impl TextureMeshRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Mesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("texture_mesh.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Texture Mesh Uniform Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Mesh Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Texture Mesh Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Texture Mesh Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
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
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            textures: Vec::new(),
            meshes: Vec::new(),
        }
    }

    pub fn load_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> usize {
        let img = image::open(path).expect(&format!("Failed to load texture: {}", path));
        let rgba = img.to_rgba8();
        let dimensions = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(path),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Mesh Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let idx = self.textures.len();
        self.textures.push((texture, view, bind_group));
        idx
    }

    pub fn add_mesh(&mut self, device: &wgpu::Device, texture_idx: usize, mesh: Mesh) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Mesh Vertex Buffer"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Texture Mesh Index Buffer"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let index_count = mesh.indices.len() as u32;
        self.meshes.push((
            texture_idx,
            vertex_buffer,
            index_buffer,
            index_count,
            mesh.transform,
        ));
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        view_proj: Mat4,
    ) {
        if self.meshes.is_empty() {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Texture Mesh Render Pass"),
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

        for (texture_idx, vertex_buffer, index_buffer, index_count, transform) in &self.meshes {
            let uniforms = Uniforms {
                view_proj: view_proj.to_cols_array_2d(),
                model: transform.to_cols_array_2d(),
            };
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

            render_pass.set_bind_group(0, &self.textures[*texture_idx].2, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..*index_count, 0, 0..1);
        }
    }
}

pub fn create_plane_mesh(width: f32, depth: f32) -> Mesh {
    let hw = width / 2.0;
    let hd = depth / 2.0;

    let vertices = vec![
        Vertex {
            position: [-hw, 0.0, -hd],
            uv: [0.0, 0.0],
        },
        Vertex {
            position: [hw, 0.0, -hd],
            uv: [width / 10.0, 0.0],
        },
        Vertex {
            position: [hw, 0.0, hd],
            uv: [width / 10.0, depth / 10.0],
        },
        Vertex {
            position: [-hw, 0.0, hd],
            uv: [0.0, depth / 10.0],
        },
    ];

    let indices = vec![0, 1, 2, 0, 2, 3];

    Mesh {
        vertices,
        indices,
        transform: Mat4::IDENTITY,
    }
}

pub fn create_cube_mesh(size: f32) -> Mesh {
    let s = size / 2.0;

    let vertices = vec![
        // Front face
        Vertex {
            position: [-s, -s, s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [s, -s, s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [s, s, s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-s, s, s],
            uv: [0.0, 0.0],
        },
        // Back face
        Vertex {
            position: [s, -s, -s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [-s, -s, -s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [-s, s, -s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [s, s, -s],
            uv: [0.0, 0.0],
        },
        // Top face
        Vertex {
            position: [-s, s, s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [s, s, s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [s, s, -s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-s, s, -s],
            uv: [0.0, 0.0],
        },
        // Bottom face
        Vertex {
            position: [-s, -s, -s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [s, -s, -s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [s, -s, s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-s, -s, s],
            uv: [0.0, 0.0],
        },
        // Right face
        Vertex {
            position: [s, -s, s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [s, -s, -s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [s, s, -s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [s, s, s],
            uv: [0.0, 0.0],
        },
        // Left face
        Vertex {
            position: [-s, -s, -s],
            uv: [0.0, 1.0],
        },
        Vertex {
            position: [-s, -s, s],
            uv: [1.0, 1.0],
        },
        Vertex {
            position: [-s, s, s],
            uv: [1.0, 0.0],
        },
        Vertex {
            position: [-s, s, -s],
            uv: [0.0, 0.0],
        },
    ];

    let indices = vec![
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    ];

    Mesh {
        vertices,
        indices,
        transform: Mat4::IDENTITY,
    }
}
