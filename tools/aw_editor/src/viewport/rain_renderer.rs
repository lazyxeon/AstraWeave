#![allow(dead_code)]

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::camera::OrbitCamera;

/// Per-vertex data (line endpoint parameter)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RainVertex {
    local_y: f32,
}

/// Per-instance data (position + speed)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RainInstance {
    position: [f32; 3],
    speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct RainUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    time: f32,
    rain_intensity: f32,
    wind_x: f32,
    wind_z: f32,
    _pad: f32,
}

const RAIN_DROP_COUNT: u32 = 8000;

pub struct RainRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    start_time: std::time::Instant,
    active: bool,
    intensity: f32,
    wind_x: f32,
    wind_z: f32,
}

impl RainRenderer {
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rain Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rain.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Rain Bind Group Layout"),
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
            label: Some("Rain Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rain Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Per-vertex buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RainVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32,
                        }],
                    },
                    // Per-instance buffer
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RainInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x3,
                            },
                            wgpu::VertexAttribute {
                                offset: 12,
                                shader_location: 2,
                                format: wgpu::VertexFormat::Float32,
                            },
                        ],
                    },
                ],
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
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
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
            label: Some("Rain Uniform Buffer"),
            contents: bytemuck::cast_slice(&[RainUniforms {
                view_proj: [[0.0; 4]; 4],
                camera_pos: [0.0; 3],
                time: 0.0,
                rain_intensity: 0.0,
                wind_x: 0.0,
                wind_z: 0.0,
                _pad: 0.0,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Rain Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Two vertices per line segment (top=0.0, bottom=1.0)
        let vertices = [RainVertex { local_y: 0.0 }, RainVertex { local_y: 1.0 }];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rain Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Generate random rain drop instances using deterministic hash
        let instances = Self::generate_instances(RAIN_DROP_COUNT);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Rain Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok(Self {
            pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            instance_buffer,
            instance_count: RAIN_DROP_COUNT,
            start_time: std::time::Instant::now(),
            active: false,
            intensity: 0.5,
            wind_x: 0.0,
            wind_z: 0.0,
        })
    }

    fn generate_instances(count: u32) -> Vec<RainInstance> {
        let mut instances = Vec::with_capacity(count as usize);
        for i in 0..count {
            // Knuth multiplicative hash for deterministic pseudo-random
            let h1 = (i.wrapping_mul(2654435761)) as f32 / u32::MAX as f32;
            let h2 = ((i.wrapping_mul(2246822519)).wrapping_add(1)) as f32 / u32::MAX as f32;
            let h3 = ((i.wrapping_mul(3266489917)).wrapping_add(2)) as f32 / u32::MAX as f32;
            let h4 = ((i.wrapping_mul(668265263)).wrapping_add(3)) as f32 / u32::MAX as f32;

            instances.push(RainInstance {
                position: [
                    (h1 - 0.5) * 80.0, // X: -40 to +40
                    (h2 - 0.5) * 80.0, // Y: -40 to +40
                    (h3 - 0.5) * 80.0, // Z: -40 to +40
                ],
                speed: 15.0 + h4 * 12.0, // 15 to 27 units/sec
            });
        }
        instances
    }

    pub fn set_active(&mut self, active: bool, intensity: f32) {
        self.active = active;
        self.intensity = intensity;
    }

    pub fn set_wind(&mut self, x: f32, z: f32) {
        self.wind_x = x;
        self.wind_z = z;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &OrbitCamera,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        if !self.active || self.intensity <= 0.0 {
            return Ok(());
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let view_proj = camera.view_projection_matrix();
        let camera_pos = camera.position();

        let uniforms = RainUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            time: elapsed,
            rain_intensity: self.intensity,
            wind_x: self.wind_x,
            wind_z: self.wind_z,
            _pad: 0.0,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Rain Render Pass"),
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
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        // 2 vertices per line, instanced N times
        pass.draw(0..2, 0..self.instance_count);

        Ok(())
    }
}
