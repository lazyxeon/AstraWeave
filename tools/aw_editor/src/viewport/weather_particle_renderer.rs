#![allow(dead_code)]

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::camera::OrbitCamera;

/// Weather type constants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WeatherKind {
    None = 0,
    Rain = 1,
    Snow = 2,
    Hail = 3,
    Sandstorm = 4,
    Blizzard = 5,
}

impl WeatherKind {
    pub fn from_weather_type(weather_type: u32) -> Self {
        match weather_type {
            0 | 1 | 5 => WeatherKind::None, // Clear, Cloudy, Fog
            2 => WeatherKind::Rain,         // Rain
            3 => WeatherKind::Rain,         // Storm (heavy rain)
            4 => WeatherKind::Snow,         // Snow
            6 => WeatherKind::Sandstorm,    // Sandstorm
            _ => WeatherKind::None,
        }
    }

    /// Map the 11-type world_panel weather to WeatherKind
    pub fn from_world_panel(weather_type: u32) -> Self {
        match weather_type {
            0..=2 | 8 => WeatherKind::None, // Clear, Cloudy, Overcast, Fog
            3 => WeatherKind::Rain,         // LightRain
            4 | 5 => WeatherKind::Rain,     // HeavyRain, Thunderstorm
            6 => WeatherKind::Snow,         // Snow
            7 => WeatherKind::Blizzard,     // Blizzard
            9 => WeatherKind::Sandstorm,    // Sandstorm
            10 => WeatherKind::Hail,        // Hail
            _ => WeatherKind::None,
        }
    }

    fn is_line_particle(self) -> bool {
        matches!(self, WeatherKind::Rain | WeatherKind::Sandstorm)
    }

    fn is_quad_particle(self) -> bool {
        matches!(
            self,
            WeatherKind::Snow | WeatherKind::Hail | WeatherKind::Blizzard
        )
    }
}

/// Weather preset parameters
struct WeatherPreset {
    particle_count: u32,
    color: [f32; 4],
    volume_size: f32,
    streak_length: f32,
    particle_scale: f32,
    base_speed: f32,
    speed_variation: f32,
}

fn preset_for(kind: WeatherKind) -> WeatherPreset {
    match kind {
        WeatherKind::None => WeatherPreset {
            particle_count: 0,
            color: [1.0, 1.0, 1.0, 0.0],
            volume_size: 40.0,
            streak_length: 0.0,
            particle_scale: 0.0,
            base_speed: 0.0,
            speed_variation: 0.0,
        },
        WeatherKind::Rain => WeatherPreset {
            particle_count: 20000,
            color: [0.7, 0.75, 0.85, 0.9],
            volume_size: 50.0,
            streak_length: 0.6,
            particle_scale: 0.0,
            base_speed: 18.0,
            speed_variation: 10.0,
        },
        WeatherKind::Snow => WeatherPreset {
            particle_count: 8000,
            color: [0.95, 0.97, 1.0, 0.85],
            volume_size: 45.0,
            streak_length: 0.0,
            particle_scale: 0.12,
            base_speed: 1.5,
            speed_variation: 1.0,
        },
        WeatherKind::Hail => WeatherPreset {
            particle_count: 10000,
            color: [0.85, 0.9, 0.95, 0.95],
            volume_size: 40.0,
            streak_length: 0.0,
            particle_scale: 0.08,
            base_speed: 12.0,
            speed_variation: 6.0,
        },
        WeatherKind::Sandstorm => WeatherPreset {
            particle_count: 6000,
            color: [0.85, 0.75, 0.55, 0.7],
            volume_size: 50.0,
            streak_length: 0.15,
            particle_scale: 0.0,
            base_speed: 4.0,
            speed_variation: 3.0,
        },
        WeatherKind::Blizzard => WeatherPreset {
            particle_count: 15000,
            color: [0.92, 0.95, 1.0, 0.9],
            volume_size: 50.0,
            streak_length: 0.0,
            particle_scale: 0.1,
            base_speed: 3.0,
            speed_variation: 2.5,
        },
    }
}

/// Per-vertex (line: local_y in .y; quad: corner in .x,.y)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParticleVertex {
    local_pos: [f32; 2],
}

/// Per-instance
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParticleInstance {
    position: [f32; 3],
    speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WeatherUniforms {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    time: f32,
    intensity: f32,
    wind_x: f32,
    wind_z: f32,
    weather_kind: f32,
    particle_color: [f32; 4],
    volume_size: f32,
    streak_length: f32,
    particle_scale: f32,
    transition_alpha: f32,
    lightning_flash: f32,
    _pad: [f32; 3],
}

pub struct WeatherParticleRenderer {
    line_pipeline: wgpu::RenderPipeline,
    quad_pipeline: wgpu::RenderPipeline,
    flash_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    // Line geometry (2 verts per line)
    line_vertex_buffer: wgpu::Buffer,
    // Quad geometry (6 verts per quad — 2 triangles)
    quad_vertex_buffer: wgpu::Buffer,
    // Shared instance buffer (regenerated per weather switch)
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
    max_instances: u32,

    start_time: std::time::Instant,
    last_frame_time: std::time::Instant,
    current_kind: WeatherKind,
    target_kind: WeatherKind,
    transition_alpha: f32,
    intensity: f32,
    wind_x: f32,
    wind_z: f32,
    active: bool,
    // Lightning flash state (storms only)
    lightning_flash: f32,
    lightning_cooldown: f32,
    lightning_rng_state: u32,
}

impl WeatherParticleRenderer {
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Weather Particle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/weather_particles.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Weather Particle BGL"),
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
            label: Some("Weather Particle Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffers = &[
            // Per-vertex
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ParticleVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                }],
            },
            // Per-instance
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ParticleInstance>() as u64,
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
        ];

        let blend_state = wgpu::BlendState {
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
        };

        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let fragment_state = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: Some(blend_state),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        // Line pipeline (rain, sandstorm)
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Weather Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(fragment_state.clone()),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Quad pipeline (snow, hail, blizzard)
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Weather Quad Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(fragment_state),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(depth_stencil.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Flash overlay pipeline (full-screen white flash for lightning)
        let flash_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Weather Flash Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_flash"),
                buffers: &[], // No vertex buffers — uses vertex_index
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_flash"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None, // No depth test for full-screen overlay
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Weather Uniform Buffer"),
            contents: bytemuck::bytes_of(&WeatherUniforms {
                view_proj: [[0.0; 4]; 4],
                camera_pos: [0.0; 3],
                time: 0.0,
                intensity: 0.0,
                wind_x: 0.0,
                wind_z: 0.0,
                weather_kind: 0.0,
                particle_color: [1.0, 1.0, 1.0, 1.0],
                volume_size: 40.0,
                streak_length: 0.3,
                particle_scale: 0.1,
                transition_alpha: 1.0,
                lightning_flash: 0.0,
                _pad: [0.0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Weather Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Line vertices: 2 endpoints per line (top=0, bottom=1)
        let line_verts = [
            ParticleVertex {
                local_pos: [0.0, 0.0],
            },
            ParticleVertex {
                local_pos: [0.0, 1.0],
            },
        ];
        let line_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Weather Line VB"),
            contents: bytemuck::cast_slice(&line_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Quad vertices: 6 verts (2 triangles) for one particle billboard
        let quad_verts = [
            ParticleVertex {
                local_pos: [0.0, 0.0],
            },
            ParticleVertex {
                local_pos: [1.0, 0.0],
            },
            ParticleVertex {
                local_pos: [0.0, 1.0],
            },
            ParticleVertex {
                local_pos: [1.0, 0.0],
            },
            ParticleVertex {
                local_pos: [1.0, 1.0],
            },
            ParticleVertex {
                local_pos: [0.0, 1.0],
            },
        ];
        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Weather Quad VB"),
            contents: bytemuck::cast_slice(&quad_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Instance buffer (max capacity)
        let max_instances = 15000u32;
        let instances = Self::generate_instances(max_instances, 40.0, 15.0, 12.0);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Weather Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            line_pipeline,
            quad_pipeline,
            flash_pipeline,
            bind_group,
            uniform_buffer,
            line_vertex_buffer,
            quad_vertex_buffer,
            instance_buffer,
            instance_count: 0,
            max_instances,
            start_time: std::time::Instant::now(),
            last_frame_time: std::time::Instant::now(),
            current_kind: WeatherKind::None,
            target_kind: WeatherKind::None,
            transition_alpha: 1.0,
            intensity: 0.5,
            wind_x: 0.0,
            wind_z: 0.0,
            active: false,
            lightning_flash: 0.0,
            lightning_cooldown: 3.0,
            lightning_rng_state: 42,
        })
    }

    fn generate_instances(
        count: u32,
        volume_size: f32,
        base_speed: f32,
        speed_variation: f32,
    ) -> Vec<ParticleInstance> {
        let mut instances = Vec::with_capacity(count as usize);
        for i in 0..count {
            let h1 = (i.wrapping_mul(2654435761)) as f32 / u32::MAX as f32;
            let h2 = ((i.wrapping_mul(2246822519)).wrapping_add(1)) as f32 / u32::MAX as f32;
            let h3 = ((i.wrapping_mul(3266489917)).wrapping_add(2)) as f32 / u32::MAX as f32;
            let h4 = ((i.wrapping_mul(668265263)).wrapping_add(3)) as f32 / u32::MAX as f32;

            instances.push(ParticleInstance {
                position: [
                    (h1 - 0.5) * volume_size * 2.0,
                    (h2 - 0.5) * volume_size * 2.0,
                    (h3 - 0.5) * volume_size * 2.0,
                ],
                speed: base_speed + h4 * speed_variation,
            });
        }
        instances
    }

    pub fn set_weather(&mut self, kind: WeatherKind, intensity: f32, queue: &wgpu::Queue) {
        if kind == self.current_kind {
            self.intensity = intensity;
            self.active = kind != WeatherKind::None;
            return;
        }

        // Start transition
        self.target_kind = kind;
        self.transition_alpha = 0.3;

        let preset = preset_for(kind);
        let count = preset.particle_count.min(self.max_instances);
        self.instance_count = count;

        // Regenerate instances with preset parameters
        let instances = Self::generate_instances(
            count,
            preset.volume_size,
            preset.base_speed,
            preset.speed_variation,
        );
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        self.intensity = intensity;
        self.active = kind != WeatherKind::None;
    }

    pub fn set_wind(&mut self, x: f32, z: f32) {
        self.wind_x = x;
        self.wind_z = z;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns the current lightning flash intensity (0.0–1.0).
    /// Can be used by the main renderer to brighten the scene during flashes.
    pub fn lightning_flash_intensity(&self) -> f32 {
        self.lightning_flash
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        camera: &OrbitCamera,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        let now = std::time::Instant::now();
        let dt = now
            .duration_since(self.last_frame_time)
            .as_secs_f32()
            .min(0.1);
        self.last_frame_time = now;

        // Advance transition (~0.2-second crossfade)
        if self.transition_alpha < 1.0 {
            self.transition_alpha = (self.transition_alpha + dt * 6.0).min(1.0);
            if self.transition_alpha >= 1.0 {
                self.current_kind = self.target_kind;
            }
        }

        let render_kind = if self.transition_alpha >= 1.0 {
            self.current_kind
        } else {
            self.target_kind
        };

        if render_kind == WeatherKind::None || self.instance_count == 0 || self.intensity <= 0.0 {
            self.lightning_flash = 0.0;
            return Ok(());
        }

        // Lightning flash logic for storms (weather_kind == Rain with high intensity)
        let is_storm = render_kind == WeatherKind::Rain && self.intensity >= 0.9;
        if is_storm {
            self.lightning_cooldown -= dt;
            if self.lightning_cooldown <= 0.0 {
                // Trigger a flash
                self.lightning_flash = 1.0;
                // Simple LCG for next interval (3-8 seconds)
                self.lightning_rng_state = self
                    .lightning_rng_state
                    .wrapping_mul(1103515245)
                    .wrapping_add(12345);
                let r = (self.lightning_rng_state >> 16) as f32 / 65535.0;
                self.lightning_cooldown = 3.0 + r * 5.0;
            }
            // Rapid decay: flash lasts ~0.15 seconds
            if self.lightning_flash > 0.0 {
                self.lightning_flash = (self.lightning_flash - dt * 8.0).max(0.0);
            }
        } else {
            self.lightning_flash = 0.0;
        }

        let preset = preset_for(render_kind);
        let elapsed = self.start_time.elapsed().as_secs_f32();
        // Camera-relative VP to avoid f32 jitter far from origin
        let view_proj = camera.view_projection_matrix_relative();
        let camera_pos = camera.position();

        let uniforms = WeatherUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            time: elapsed,
            intensity: self.intensity,
            wind_x: self.wind_x,
            wind_z: self.wind_z,
            weather_kind: render_kind as u32 as f32,
            particle_color: preset.color,
            volume_size: preset.volume_size,
            streak_length: preset.streak_length,
            particle_scale: preset.particle_scale,
            transition_alpha: self.transition_alpha,
            lightning_flash: self.lightning_flash,
            _pad: [0.0; 3],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Weather Particle Pass"),
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

        // Choose pipeline based on particle type
        if render_kind.is_line_particle() {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.line_vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.draw(0..2, 0..self.instance_count);
        } else if render_kind.is_quad_particle() {
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            pass.draw(0..6, 0..self.instance_count);
        }

        drop(pass);

        // Lightning flash overlay (full-screen white flash during storms)
        if self.lightning_flash > 0.01 {
            let mut flash_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Lightning Flash Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            flash_pass.set_pipeline(&self.flash_pipeline);
            flash_pass.set_bind_group(0, &self.bind_group, &[]);
            flash_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}
