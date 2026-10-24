//! Scene normal and motion, written into the gated aux targets.
//!
//! Off unless `Renderer::set_scene_vectors_enabled(true)`, and only `draw_into`
//! records it. The pass does not enable SSGI, TAA, or temporal upscale, and
//! it does not write simulation state. `deferred.rs` keeps its own G-buffer.
//! The stored normal is view space packed into [0, 1]. The stored velocity
//! is a framebuffer UV delta (Y down). SSGI, TAA, and temporal upscale still
//! do not copy their output onto the presented HDR target, and their history
//! textures stay unwired. Enabling this pass does not make those inputs ready.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Matrices {
    curr_vp: [[f32; 4]; 4],
    prev_vp: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    alpha_cutoff: f32,
    _pad: [f32; 3],
}

pub struct SceneVectorsPass {
    pipeline: wgpu::RenderPipeline,
    terrain_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    /// Bound uniform. Updated by `copy_uniform` so a weather cutoff of 0 and
    /// the mesh cutoff stay ordered with the passes that read them.
    matrices: wgpu::Buffer,
    weather_uniform: wgpu::Buffer,
    mesh_uniform: wgpu::Buffer,
}

fn instance_stride() -> wgpu::BufferAddress {
    std::mem::size_of::<crate::types::InstanceRaw>() as wgpu::BufferAddress
}

pub fn current_model_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: instance_stride(),
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &CURRENT_MODEL,
    }
}

pub fn prev_instance_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: instance_stride(),
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &PREV_MODEL,
    }
}

const CURRENT_MODEL: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        offset: 0,
        shader_location: 2,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 16,
        shader_location: 3,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 32,
        shader_location: 4,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 48,
        shader_location: 5,
        format: wgpu::VertexFormat::Float32x4,
    },
];

const TERRAIN_PREV_POSITION: wgpu::VertexAttribute = wgpu::VertexAttribute {
    offset: 0,
    shader_location: 3,
    format: wgpu::VertexFormat::Float32x3,
};

/// Previous terrain positions live in a copy of the splat vertex buffer.
/// The stride matches `TerrainSplatVertex` so the copy can be bound as-is.
const TERRAIN_PREV_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<crate::terrain_material_manager::TerrainSplatVertex>()
        as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[TERRAIN_PREV_POSITION],
};

const PREV_MODEL: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        offset: 0,
        shader_location: 6,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 16,
        shader_location: 7,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 32,
        shader_location: 8,
        format: wgpu::VertexFormat::Float32x4,
    },
    wgpu::VertexAttribute {
        offset: 48,
        shader_location: 10,
        format: wgpu::VertexFormat::Float32x4,
    },
];

impl SceneVectorsPass {
    pub fn new(device: &wgpu::Device, tex_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene vectors"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/scene_vectors.wgsl").into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene vectors matrices"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let uniform_size = std::mem::size_of::<Matrices>() as u64;
        let matrices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene vectors matrices"),
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scratch = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: uniform_size,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let weather_uniform = scratch("scene vectors weather uniform");
        let mesh_uniform = scratch("scene vectors mesh uniform");
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene vectors matrices"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: matrices.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene vectors"),
            bind_group_layouts: &[&bind_layout, tex_layout],
            push_constant_ranges: &[],
        });
        let terrain_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene vectors terrain"),
            bind_group_layouts: &[&bind_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("scene vectors"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[
                    crate::types::Vertex::layout(),
                    current_model_layout(),
                    prev_instance_layout(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rg16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                // Less matches the color pass. A constant bias of -2 lets the
                // same triangles pass against the depth they already wrote.
                // slope_scale stays 0: a negative slope pulls in triangles
                // that are farther than the color pass kept. Equal-depth
                // overlaps are drawn in reverse color order, so the last
                // fragment here is the one the color pass kept.
                depth_compare: wgpu::CompareFunction::Less,
                bias: wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
                stencil: wgpu::StencilState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let terrain_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("scene vectors terrain"),
            layout: Some(&terrain_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_terrain"),
                buffers: &[
                    crate::terrain_material_manager::TerrainSplatVertex::LAYOUT,
                    TERRAIN_PREV_LAYOUT,
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_terrain"),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rg16Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                // Less matches the color pass. A constant bias of -2 lets the
                // same triangles pass against the depth they already wrote.
                // slope_scale stays 0: a negative slope pulls in triangles
                // that are farther than the color pass kept. Equal-depth
                // overlaps are drawn in reverse color order, so the last
                // fragment here is the one the color pass kept.
                depth_compare: wgpu::CompareFunction::Less,
                bias: wgpu::DepthBiasState {
                    constant: -2,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
                stencil: wgpu::StencilState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        Self {
            pipeline,
            terrain_pipeline,
            bind_group,
            matrices,
            weather_uniform,
            mesh_uniform,
        }
    }

    fn fill_uniform(
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        curr: glam::Mat4,
        prev: glam::Mat4,
        view: glam::Mat4,
        alpha_cutoff: f32,
    ) {
        let matrices = Matrices {
            curr_vp: curr.to_cols_array_2d(),
            prev_vp: prev.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            alpha_cutoff,
            _pad: [0.0; 3],
        };
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&matrices));
    }

    /// Queue-write the weather cutoff (0) and the mesh cutoff into staging
    /// buffers. [`Self::copy_uniform`] puts one of them into the bound
    /// uniform inside the encoder, so the later write does not replace the
    /// earlier one before `submit`.
    pub fn stage_uniforms(
        &self,
        queue: &wgpu::Queue,
        curr: glam::Mat4,
        prev: glam::Mat4,
        view: glam::Mat4,
        mesh_cutoff: f32,
    ) {
        Self::fill_uniform(queue, &self.weather_uniform, curr, prev, view, 0.0);
        Self::fill_uniform(queue, &self.mesh_uniform, curr, prev, view, mesh_cutoff);
    }

    pub fn copy_weather_uniform(&self, enc: &mut wgpu::CommandEncoder) {
        self.copy_uniform(enc, &self.weather_uniform);
    }

    pub fn copy_mesh_uniform(&self, enc: &mut wgpu::CommandEncoder) {
        self.copy_uniform(enc, &self.mesh_uniform);
    }

    fn copy_uniform(&self, enc: &mut wgpu::CommandEncoder, src: &wgpu::Buffer) {
        let bytes = std::mem::size_of::<Matrices>() as u64;
        enc.copy_buffer_to_buffer(src, 0, &self.matrices, 0, bytes);
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }

    pub fn terrain_pipeline(&self) -> &wgpu::RenderPipeline {
        &self.terrain_pipeline
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
