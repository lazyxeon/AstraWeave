#![allow(dead_code)]

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::camera::OrbitCamera;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterVertex {
    position: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WaterUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    time: f32,
    fog_color: [f32; 3],
    fog_density: f32,
    water_level: f32,
    fog_enabled: u32,
    sun_dir: [f32; 3],
    sun_intensity: f32,
}

pub struct WaterRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    start_time: std::time::Instant,
    water_level: f32,
    enabled: bool,
    fog_enabled: bool,
    fog_density: f32,
    fog_color: [f32; 3],
    sun_dir: [f32; 3],
    sun_intensity: f32,
}

impl WaterRenderer {
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Water Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/water.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Water Bind Group Layout"),
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Water Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Water Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<WaterVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Render both sides of water
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // Transparent — don't write depth
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

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Uniform Buffer"),
            contents: bytemuck::cast_slice(&[WaterUniforms {
                view_proj: [[0.0; 4]; 4],
                camera_pos: [0.0; 3],
                time: 0.0,
                fog_color: [0.6, 0.6, 0.62],
                fog_density: 0.01,
                water_level: 0.0,
                fog_enabled: 0,
                sun_dir: [0.5, 0.7, 0.35],
                sun_intensity: 1.6,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Water Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Generate water grid mesh (128x128, spanning -200..200)
        let (vertices, indices) = Self::generate_water_mesh(128, 200.0);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Water Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
            start_time: std::time::Instant::now(),
            water_level: 0.0,
            enabled: false,
            fog_enabled: false,
            fog_density: 0.01,
            fog_color: [0.6, 0.6, 0.62],
            sun_dir: [0.5, 0.7, 0.35],
            sun_intensity: 1.6,
        })
    }

    fn generate_water_mesh(resolution: u32, extent: f32) -> (Vec<WaterVertex>, Vec<u32>) {
        let res = resolution + 1;
        let mut vertices = Vec::with_capacity((res * res) as usize);
        let mut indices = Vec::with_capacity(((resolution) * (resolution) * 6) as usize);

        for z in 0..res {
            for x in 0..res {
                let fx = (x as f32 / resolution as f32) * 2.0 * extent - extent;
                let fz = (z as f32 / resolution as f32) * 2.0 * extent - extent;
                vertices.push(WaterVertex {
                    position: [fx, 0.0, fz],
                });
            }
        }

        for z in 0..resolution {
            for x in 0..resolution {
                let tl = z * res + x;
                let tr = tl + 1;
                let bl = (z + 1) * res + x;
                let br = bl + 1;
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }

        (vertices, indices)
    }

    pub fn set_water_level(&mut self, level: f32) {
        self.water_level = level;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_fog(&mut self, enabled: bool, density: f32, color: [f32; 3]) {
        self.fog_enabled = enabled;
        self.fog_density = density;
        self.fog_color = color;
    }

    pub fn set_sun(&mut self, dir: [f32; 3], intensity: f32) {
        self.sun_dir = dir;
        self.sun_intensity = intensity;
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &OrbitCamera,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        // Camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let camera_pos = camera.position();

        let uniforms = WaterUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            time: elapsed,
            fog_color: self.fog_color,
            fog_density: self.fog_density,
            water_level: self.water_level,
            fog_enabled: if self.fog_enabled { 1 } else { 0 },
            sun_dir: self.sun_dir,
            sun_intensity: self.sun_intensity,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Water Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);

        Ok(())
    }
}
