//! Environmental rendering systems including sky, weather, and time-of-day
//!
//! This module provides rendering for atmospheric and environmental effects
//! that enhance the biome experience in AstraWeave.

use anyhow::{Context, Result};
use glam::{vec3, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Instant;
use wgpu::util::DeviceExt;

/// Time of day system that manages sun/moon position and lighting
#[derive(Debug, Clone)]
pub struct TimeOfDay {
    /// Current time in game hours (0.0 - 24.0)
    pub current_time: f32,
    /// How fast time progresses (1.0 = real time, 60.0 = 1 minute = 1 hour)
    pub time_scale: f32,
    /// When the day started in real time
    start_time: Instant,
    /// Length of a full day-night cycle in real seconds
    pub day_length: f32,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            current_time: 12.0, // Start at noon
            time_scale: 60.0,   // 1 real minute = 1 game hour
            start_time: Instant::now(),
            day_length: 1440.0, // 24 real minutes = 1 game day
        }
    }
}

impl TimeOfDay {
    /// Create a new time system starting at the specified time
    pub fn new(start_time: f32, time_scale: f32) -> Self {
        Self {
            current_time: start_time,
            time_scale,
            start_time: Instant::now(),
            day_length: 1440.0,
        }
    }

    /// Update the time based on real elapsed time
    pub fn update(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let game_hours = (elapsed * self.time_scale) / 3600.0;
        self.current_time = (self.current_time + game_hours) % 24.0;
        self.start_time = Instant::now();
    }

    /// Get the sun position in the sky (-1 to 1 range)
    pub fn get_sun_position(&self) -> Vec3 {
        // Sun rises at 6am, peaks at noon, sets at 6pm
        let sun_angle = (self.current_time - 6.0) * std::f32::consts::PI / 12.0;
        let sun_height = sun_angle.sin();
        let sun_azimuth = (self.current_time - 12.0) * std::f32::consts::PI / 12.0;

        // At noon (12:00), sun should be nearly overhead (0, 1, 0)
        // At sunrise/sunset (6:00/18:00), sun should be at horizon
        // At midnight (0:00), sun should be below horizon (0, -1, 0)

        if sun_height.abs() < 0.01 {
            // Near horizon, avoid division by zero
            vec3(sun_azimuth.sin(), 0.0, sun_azimuth.cos()).normalize()
        } else {
            let horizontal_distance = (1.0 - sun_height.abs()).max(0.1);
            vec3(
                sun_azimuth.sin() * horizontal_distance,
                sun_height,
                sun_azimuth.cos() * horizontal_distance,
            )
            .normalize()
        }
    }

    /// Get the moon position in the sky (opposite to sun)
    pub fn get_moon_position(&self) -> Vec3 {
        -self.get_sun_position()
    }

    /// Get the primary light direction (sun during day, moon at night)
    pub fn get_light_direction(&self) -> Vec3 {
        let sun_pos = self.get_sun_position();
        if sun_pos.y > 0.1 {
            -sun_pos // Light comes from the sun
        } else {
            -self.get_moon_position() // Light comes from the moon
        }
    }

    /// Get the light color based on time of day
    pub fn get_light_color(&self) -> Vec3 {
        let sun_height = self.get_sun_position().y;

        if sun_height > 0.2 {
            // Daytime - warm yellow/white sunlight
            let intensity = (sun_height - 0.2) / 0.8;
            vec3(1.0, 0.95, 0.8) * (0.8 + 0.2 * intensity)
        } else if sun_height > -0.2 {
            // Twilight - orange/red sunset/sunrise
            let intensity = (sun_height + 0.2) / 0.4;
            vec3(1.0, 0.6, 0.3) * (0.3 + 0.5 * intensity)
        } else {
            // Night - cool blue moonlight
            vec3(0.3, 0.4, 0.8) * 0.15
        }
    }

    /// Get ambient light color
    pub fn get_ambient_color(&self) -> Vec3 {
        let sun_height = self.get_sun_position().y;

        if sun_height > 0.0 {
            // Day ambient - bright blue sky
            let intensity = sun_height.min(1.0);
            vec3(0.4, 0.6, 1.0) * (0.3 + 0.4 * intensity)
        } else {
            // Night ambient - dark blue
            vec3(0.1, 0.15, 0.3) * 0.1
        }
    }

    /// Check if it's currently day time
    pub fn is_day(&self) -> bool {
        self.get_sun_position().y > 0.0
    }

    /// Check if it's currently night time
    pub fn is_night(&self) -> bool {
        self.get_sun_position().y < -0.1
    }

    /// Check if it's currently twilight (sunrise/sunset)
    pub fn is_twilight(&self) -> bool {
        let sun_height = self.get_sun_position().y;
        (-0.1..=0.1).contains(&sun_height)
    }
}

/// Sky rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyConfig {
    /// Sky colors for different times of day
    pub day_color_top: Vec3,
    pub day_color_horizon: Vec3,
    pub sunset_color_top: Vec3,
    pub sunset_color_horizon: Vec3,
    pub night_color_top: Vec3,
    pub night_color_horizon: Vec3,
    /// Cloud configuration
    pub cloud_coverage: f32,
    pub cloud_speed: f32,
    pub cloud_altitude: f32,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self {
            day_color_top: vec3(0.3, 0.6, 1.0),
            day_color_horizon: vec3(0.8, 0.9, 1.0),
            sunset_color_top: vec3(0.8, 0.4, 0.2),
            sunset_color_horizon: vec3(1.0, 0.6, 0.3),
            night_color_top: vec3(0.0, 0.0, 0.1),
            night_color_horizon: vec3(0.1, 0.1, 0.2),
            cloud_coverage: 0.5,
            cloud_speed: 0.02,
            cloud_altitude: 1000.0,
        }
    }
}

/// Sky rendering system
pub struct SkyRenderer {
    config: SkyConfig,
    time_of_day: TimeOfDay,
    // GPU resources for sky rendering
    skybox_pipeline: Option<wgpu::RenderPipeline>,
    skybox_vertices: Option<wgpu::Buffer>,
    skybox_indices: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    bind_group: Option<wgpu::BindGroup>,
    // Textured skybox support
    texture_pipeline: Option<wgpu::RenderPipeline>,
    texture_bgl: Option<wgpu::BindGroupLayout>,
    equirect_pipeline: Option<wgpu::RenderPipeline>,
    equirect_bgl: Option<wgpu::BindGroupLayout>,
    index_count: u32,
}

impl SkyRenderer {
    /// Create a new sky renderer
    pub fn new(config: SkyConfig) -> Self {
        Self {
            config,
            time_of_day: TimeOfDay::default(),
            skybox_pipeline: None,
            skybox_vertices: None,
            skybox_indices: None,
            uniform_buffer: None,
            bind_group: None,
            texture_pipeline: None,
            texture_bgl: None,
            equirect_pipeline: None,
            equirect_bgl: None,
            index_count: 0,
        }
    }

    /// Initialize GPU resources for sky rendering
    pub fn init_gpu_resources(
        &mut self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<()> {
        // Create skybox geometry (inverted cube)
        let vertices = self.create_skybox_vertices();
        let indices = self.create_skybox_indices();
        self.index_count = indices.len() as u32;

        self.skybox_vertices = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Skybox Vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        self.skybox_indices = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Skybox Indices"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
        );

        // Create uniform buffer for sky parameters
        self.uniform_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sky Uniform Buffer"),
            size: std::mem::size_of::<SkyUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Create bind group layout and bind group
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sky Bind Group Layout"),
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

        self.bind_group = Some(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Sky Bind Group"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self
                        .uniform_buffer
                        .as_ref()
                        // INVARIANT: uniform_buffer set to Some at L242 above
                        .map(|b| b.as_entire_binding())
                        .unwrap_or_else(|| unreachable!()),
                }],
            }),
        );

        // Create render pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SKY_SHADER_SOURCE)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sky Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        self.skybox_pipeline = Some(device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Sky Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[SkyVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // No culling for skybox
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false, // Skybox should not write depth
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));

        // Create textured skybox pipeline
        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Textured Sky BGL"),
            entries: &[
                // Uniforms (ViewProj)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Texture Cube
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Textured Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TEXTURED_SKY_SHADER_SOURCE)),
        });

        let texture_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Textured Sky Pipeline Layout"),
            bind_group_layouts: &[&texture_bgl],
            push_constant_ranges: &[],
        });

        self.texture_pipeline = Some(device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Textured Sky Pipeline"),
                layout: Some(&texture_pl),
                vertex: wgpu::VertexState {
                    module: &texture_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[SkyVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &texture_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));
        self.texture_bgl = Some(texture_bgl);

        // Initialize Equirect pipeline
        let equirect_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Equirect Sky BGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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

        let equirect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Equirect Sky Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(EQUIRECT_SKY_SHADER_SOURCE)),
        });

        let equirect_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Equirect Sky Layout"),
            bind_group_layouts: &[&equirect_bgl],
            push_constant_ranges: &[],
        });

        self.equirect_pipeline = Some(device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Equirect Sky Pipeline"),
                layout: Some(&equirect_pl),
                vertex: wgpu::VertexState {
                    module: &equirect_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[SkyVertex::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &equirect_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            },
        ));
        self.equirect_bgl = Some(equirect_bgl);

        Ok(())
    }

    /// Update the time of day and sky state
    pub fn update(&mut self, _delta_time: f32) {
        self.time_of_day.update();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        view_proj: Mat4,
        queue: &wgpu::Queue,
        sky_texture: Option<&wgpu::TextureView>,
        equirect_texture: Option<&wgpu::TextureView>,
    ) -> Result<()> {
        let pipeline = self
            .skybox_pipeline
            .as_ref()
            .context("Sky pipeline not initialized")?;

        // Update uniforms
        let uniforms = self.create_sky_uniforms(view_proj);
        let uniform_buffer = self
            .uniform_buffer
            .as_ref()
            .context("Uniform buffer not initialized")?;

        let data = bytemuck::bytes_of(&uniforms);
        queue.write_buffer(uniform_buffer, 0, data);

        let vertices = self
            .skybox_vertices
            .as_ref()
            .context("Vertices not initialized")?;
        let indices = self
            .skybox_indices
            .as_ref()
            .context("Indices not initialized")?;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sky Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // In debug builds use magenta so a missing skybox is
                    // immediately visible; in release use dark grey.
                    load: wgpu::LoadOp::Clear(if cfg!(debug_assertions) {
                        wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 1.0,
                            a: 1.0,
                        }
                    } else {
                        wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }
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

        // Render procedural, textured (cube), or equirect skybox
        let mut drawn = false;

        // 1. Try Equirectangular (Preferred if available)
        if let Some(eqr_tex) = equirect_texture {
            if let (Some(eqr_pipeline), Some(eqr_bgl)) =
                (&self.equirect_pipeline, &self.equirect_bgl)
            {
                let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("Skybox Eqr Sampler"),
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
                    address_mode_u: wgpu::AddressMode::Repeat, // Wrap horizontally
                    address_mode_v: wgpu::AddressMode::ClampToEdge, // Clamp poles
                    ..Default::default()
                });

                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Sky Eqr BG"),
                    layout: eqr_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(eqr_tex),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                    ],
                });

                render_pass.set_pipeline(eqr_pipeline);
                render_pass.set_bind_group(0, &bg, &[]);
                render_pass.set_vertex_buffer(0, vertices.slice(..));
                render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                drawn = true;
            }
        }

        // 2. Try Textured Cube (Fallback)
        if !drawn {
            if let Some(tex) = sky_texture {
                if let (Some(tex_pipeline), Some(tex_bgl)) =
                    (&self.texture_pipeline, &self.texture_bgl)
                {
                    // Create sampler
                    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some("Skybox Sampler"),
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    });

                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Sky Texture BG"),
                        layout: tex_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: uniform_buffer.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(tex),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                    });

                    render_pass.set_pipeline(tex_pipeline);
                    render_pass.set_bind_group(0, &bg, &[]);
                    render_pass.set_vertex_buffer(0, vertices.slice(..));
                    render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..self.index_count, 0, 0..1);
                    drawn = true;
                }
            }
        }

        // 3. Procedural (Fallback)
        if !drawn {
            render_pass.set_pipeline(pipeline);
            let bind_group = self
                .bind_group
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("bind_group must be initialized before render"))?;
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertices.slice(..));
            render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        Ok(())
    }

    /// Get the current time of day system
    pub fn time_of_day(&self) -> &TimeOfDay {
        &self.time_of_day
    }

    /// Get mutable reference to time of day system
    pub fn time_of_day_mut(&mut self) -> &mut TimeOfDay {
        &mut self.time_of_day
    }

    /// Get current sky configuration
    pub fn config(&self) -> &SkyConfig {
        &self.config
    }

    /// Update sky configuration
    pub fn set_config(&mut self, config: SkyConfig) {
        self.config = config;
    }

    // Private helper methods

    fn create_skybox_vertices(&self) -> Vec<SkyVertex> {
        // Create a sphere for skybox to avoid corner artifacts
        let rings = 16;
        let segments = 32;
        let mut vertices = Vec::new();
        for r in 0..=rings {
            let phi = std::f32::consts::PI * (r as f32 / rings as f32);
            for s in 0..=segments {
                let theta = std::f32::consts::PI * 2.0 * (s as f32 / segments as f32);
                // Y-up spherical coordinates, Radius 500.0 to avoid near-plane clipping
                let r = 500.0;
                let x = r * phi.sin() * theta.cos();
                let y = r * phi.cos();
                let z = r * phi.sin() * theta.sin();
                vertices.push(SkyVertex {
                    position: [x, y, z],
                });
            }
        }
        vertices
    }

    fn create_skybox_indices(&self) -> Vec<u16> {
        let rings = 16;
        let segments = 32;
        let mut indices = Vec::new();
        for r in 0..rings {
            for s in 0..segments {
                let current = r * (segments + 1) + s;
                let next = r * (segments + 1) + (s + 1);
                let bottom = (r + 1) * (segments + 1) + s;
                let bottom_next = (r + 1) * (segments + 1) + (s + 1);

                // Triangle 1
                indices.push(current as u16);
                indices.push(bottom as u16);
                indices.push(next as u16);

                // Triangle 2
                indices.push(bottom as u16);
                indices.push(bottom_next as u16);
                indices.push(next as u16);
            }
        }
        indices
    }

    fn create_sky_uniforms(&self, view_proj: Mat4) -> SkyUniforms {
        let sun_pos = self.time_of_day.get_sun_position();
        let moon_pos = self.time_of_day.get_moon_position();

        // Interpolate sky colors based on time of day
        let (top_color, horizon_color) = if self.time_of_day.is_day() {
            (self.config.day_color_top, self.config.day_color_horizon)
        } else if self.time_of_day.is_night() {
            (self.config.night_color_top, self.config.night_color_horizon)
        } else {
            // Twilight - blend between day and sunset
            let sun_height = sun_pos.y;
            let t = (sun_height + 0.1) / 0.2; // Map -0.1 to 0.1 -> 0.0 to 1.0
            let t = t.clamp(0.0, 1.0);

            let top = self
                .config
                .sunset_color_top
                .lerp(self.config.day_color_top, t);
            let horizon = self
                .config
                .sunset_color_horizon
                .lerp(self.config.day_color_horizon, t);
            (top, horizon)
        };

        SkyUniforms {
            view_proj: view_proj.to_cols_array(),
            sun_position: [sun_pos.x, sun_pos.y, sun_pos.z, 0.0],
            moon_position: [moon_pos.x, moon_pos.y, moon_pos.z, 0.0],
            top_color: [top_color.x, top_color.y, top_color.z, 1.0],
            horizon_color: [horizon_color.x, horizon_color.y, horizon_color.z, 1.0],
            time_of_day: self.time_of_day.current_time,
            cloud_coverage: self.config.cloud_coverage,
            cloud_speed: self.config.cloud_speed,
            _padding: 0.0,
        }
    }
}

/// Vertex format for skybox rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyVertex {
    position: [f32; 3],
}

impl SkyVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}

/// Uniform data for sky rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniforms {
    view_proj: [f32; 16],
    sun_position: [f32; 4],
    moon_position: [f32; 4],
    top_color: [f32; 4],
    horizon_color: [f32; 4],
    time_of_day: f32,
    cloud_coverage: f32,
    cloud_speed: f32,
    _padding: f32,
}

/// WGSL shader source for sky rendering
const SKY_SHADER_SOURCE: &str = r#"
struct SkyUniforms {
    view_proj: mat4x4<f32>,
    sun_position: vec4<f32>,
    moon_position: vec4<f32>,
    top_color: vec4<f32>,
    horizon_color: vec4<f32>,
    time_of_day: f32,
    cloud_coverage: f32,
    cloud_speed: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: SkyUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Remove translation from view matrix to make skybox always centered on camera
    var view_no_translation = uniforms.view_proj;
    view_no_translation[3][0] = 0.0;
    view_no_translation[3][1] = 0.0;
    view_no_translation[3][2] = 0.0;
    
    out.clip_position = view_no_translation * vec4<f32>(input.position, 1.0);
    // Force z = w to place skybox at maximum depth (far plane)
    // This ensures skybox always passes depth test with LessEqual compare
    out.clip_position.z = out.clip_position.w;
    out.world_position = input.position;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let direction = normalize(input.world_position);
    
    // Calculate gradient from horizon to top based on Y component
    let height_factor = (direction.y + 1.0) * 0.5; // Map -1,1 to 0,1
    let sky_color = mix(uniforms.horizon_color.rgb, uniforms.top_color.rgb, height_factor);
    
    // Simple sun/moon disc rendering
    let sun_dot = dot(direction, uniforms.sun_position.xyz);
    let moon_dot = dot(direction, uniforms.moon_position.xyz);
    
    var final_color = sky_color;
    
    // Add sun disc
    if (sun_dot > 0.999 && uniforms.sun_position.y > 0.0) {
        final_color = mix(final_color, vec3<f32>(1.0, 1.0, 0.8), 0.8);
    }
    
    // Add moon disc
    if (moon_dot > 0.998 && uniforms.moon_position.y > 0.0) {
        final_color = mix(final_color, vec3<f32>(0.9, 0.9, 1.0), 0.6);
    }
    
    // Simple cloud effect using noise-like function
    let cloud_coord = direction.xz * 5.0 + uniforms.cloud_speed * uniforms.time_of_day * 0.1;
    let cloud_noise = sin(cloud_coord.x * 3.14159) * cos(cloud_coord.y * 2.718) * 0.5 + 0.5;
    let cloud_factor = smoothstep(1.0 - uniforms.cloud_coverage, 1.0, cloud_noise);
    
    if (direction.y > 0.2) { // Only show clouds above horizon
        final_color = mix(final_color, vec3<f32>(0.9, 0.9, 1.0), cloud_factor * 0.7);
    }
    
    return vec4<f32>(final_color, 1.0);
}
"#;

const TEXTURED_SKY_SHADER_SOURCE: &str = r#"
struct SkyUniforms {
    view_proj: mat4x4<f32>,
    sun_position: vec4<f32>,
    moon_position: vec4<f32>,
    top_color: vec4<f32>,
    horizon_color: vec4<f32>,
    time_of_day: f32,
    cloud_coverage: f32,
    cloud_speed: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> uniforms: SkyUniforms;
@group(0) @binding(1) var sky_tex: texture_cube<f32>;
@group(0) @binding(2) var sky_samp: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Remove translation from view matrix to make skybox always centered on camera
    var view_no_translation = uniforms.view_proj;
    view_no_translation[3][0] = 0.0;
    view_no_translation[3][1] = 0.0;
    view_no_translation[3][2] = 0.0;
    
    out.clip_position = view_no_translation * vec4<f32>(input.position, 1.0);
    out.clip_position.z = out.clip_position.w; // Force max depth
    out.world_position = input.position;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Correct for world space vs texture space if needed.
    // Usually cubemaps Y-up match world Y-up.
    // Ensure we normalize
    let dir = normalize(input.world_position);
    let color = textureSample(sky_tex, sky_samp, dir);
    // Apply tonemapping?? Renderer main pass does tonemapping.
    // Skybox should be HDR values.
    return color;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_of_day_creation() {
        let time = TimeOfDay::new(12.0, 60.0);
        assert_eq!(time.current_time, 12.0);
        assert_eq!(time.time_scale, 60.0);
    }

    #[test]
    fn test_sun_position_calculation() {
        let mut time = TimeOfDay::new(12.0, 1.0); // Noon
        let sun_pos = time.get_sun_position();
        assert!(sun_pos.y > 0.9); // Should be high in sky at noon

        time.current_time = 6.0; // Sunrise
        let sun_pos = time.get_sun_position();
        assert!(sun_pos.y < 0.1); // Should be at horizon

        time.current_time = 0.0; // Midnight
        let sun_pos = time.get_sun_position();
        assert!(sun_pos.y < 0.0); // Should be below horizon
    }

    #[test]
    fn test_day_night_detection() {
        let mut time = TimeOfDay::new(12.0, 1.0);
        assert!(time.is_day());
        assert!(!time.is_night());

        time.current_time = 0.0; // Midnight
        assert!(!time.is_day());
        assert!(time.is_night());

        time.current_time = 6.0; // Sunrise
        assert!(time.is_twilight());
    }

    #[test]
    fn test_sky_config_default() {
        let config = SkyConfig::default();
        assert_eq!(config.cloud_coverage, 0.5);
        assert_eq!(config.cloud_speed, 0.02);
    }

    #[test]
    fn test_weather_system() {
        let mut weather = WeatherSystem::new();
        assert_eq!(weather.current_weather(), WeatherType::Clear);

        weather.set_weather(WeatherType::Rain, 0.0);
        assert_eq!(weather.current_weather(), WeatherType::Rain);
        assert!(weather.get_rain_intensity() > 0.0);
    }
}

/// Types of weather that can occur
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WeatherType {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Snow,
    Fog,
    Sandstorm, // For desert biomes
}

/// Weather system that manages weather transitions and effects
#[derive(Debug, Clone)]
pub struct WeatherSystem {
    current_weather: WeatherType,
    target_weather: WeatherType,
    transition_duration: f32,
    transition_progress: f32,
    rain_intensity: f32,
    snow_intensity: f32,
    fog_density: f32,
    wind_strength: f32,
    wind_direction: Vec3,
    last_update: Instant,
}

impl Default for WeatherSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherSystem {
    /// Create a new weather system starting with clear weather
    pub fn new() -> Self {
        Self {
            current_weather: WeatherType::Clear,
            target_weather: WeatherType::Clear,
            transition_duration: 30.0, // 30 seconds default transition
            transition_progress: 1.0,
            rain_intensity: 0.0,
            snow_intensity: 0.0,
            fog_density: 0.0,
            wind_strength: 0.1,
            wind_direction: vec3(1.0, 0.0, 0.0),
            last_update: Instant::now(),
        }
    }

    /// Update the weather system
    pub fn update(&mut self, delta_time: f32) {
        self.last_update = Instant::now();

        // Update transition if needed
        if self.current_weather != self.target_weather {
            self.transition_progress += delta_time / self.transition_duration;

            if self.transition_progress >= 1.0 {
                self.current_weather = self.target_weather;
                self.transition_progress = 1.0;
            }
        }

        // Update weather parameters based on current state
        self.update_weather_parameters();

        // Animate wind direction slightly
        let wind_variation = (self.last_update.elapsed().as_secs_f32() * 0.5).sin() * 0.1;
        self.wind_direction = vec3(
            (1.0 + wind_variation).cos(),
            0.0,
            (1.0 + wind_variation).sin(),
        )
        .normalize();
    }

    /// Set target weather with transition duration
    pub fn set_weather(&mut self, weather: WeatherType, transition_duration: f32) {
        if weather != self.current_weather {
            self.target_weather = weather;
            if transition_duration <= 0.0 {
                // Instant weather change
                self.current_weather = weather;
                self.transition_progress = 1.0;
                self.update_weather_parameters();
            } else {
                // Gradual transition
                self.transition_duration = transition_duration;
                self.transition_progress = 0.0;
            }
        }
    }

    /// Get the current weather type
    pub fn current_weather(&self) -> WeatherType {
        self.current_weather
    }

    /// Get the target weather type
    pub fn target_weather(&self) -> WeatherType {
        self.target_weather
    }

    /// Get current rain intensity (0.0 to 1.0)
    pub fn get_rain_intensity(&self) -> f32 {
        self.rain_intensity
    }

    /// Get current snow intensity (0.0 to 1.0)
    pub fn get_snow_intensity(&self) -> f32 {
        self.snow_intensity
    }

    /// Get current fog density (0.0 to 1.0)
    pub fn get_fog_density(&self) -> f32 {
        self.fog_density
    }

    /// Get current wind strength
    pub fn get_wind_strength(&self) -> f32 {
        self.wind_strength
    }

    /// Get current wind direction
    pub fn get_wind_direction(&self) -> Vec3 {
        self.wind_direction
    }

    /// Check if it's currently raining
    pub fn is_raining(&self) -> bool {
        matches!(self.current_weather, WeatherType::Rain | WeatherType::Storm)
            && self.rain_intensity > 0.1
    }

    /// Check if it's currently snowing
    pub fn is_snowing(&self) -> bool {
        self.current_weather == WeatherType::Snow && self.snow_intensity > 0.1
    }

    /// Check if there's fog
    pub fn is_foggy(&self) -> bool {
        self.fog_density > 0.1
    }

    /// Get weather-based terrain color modifier
    pub fn get_terrain_color_modifier(&self) -> Vec3 {
        match self.current_weather {
            WeatherType::Clear => vec3(1.0, 1.0, 1.0),
            WeatherType::Cloudy => vec3(0.8, 0.8, 0.9),
            WeatherType::Rain | WeatherType::Storm => {
                // Wet terrain is darker
                let wetness = self.rain_intensity * 0.7;
                vec3(
                    1.0 - wetness * 0.3,
                    1.0 - wetness * 0.2,
                    1.0 - wetness * 0.1,
                )
            }
            WeatherType::Snow => {
                // Snow makes terrain whiter
                let snow_cover = self.snow_intensity * 0.8;
                vec3(
                    1.0 + snow_cover * 0.5,
                    1.0 + snow_cover * 0.5,
                    1.0 + snow_cover * 0.6,
                )
            }
            WeatherType::Fog => vec3(0.9, 0.9, 1.0),
            WeatherType::Sandstorm => vec3(1.0, 0.8, 0.6),
        }
    }

    /// Get light attenuation due to weather
    pub fn get_light_attenuation(&self) -> f32 {
        match self.current_weather {
            WeatherType::Clear => 1.0,
            WeatherType::Cloudy => 0.7,
            WeatherType::Rain => 0.5,
            WeatherType::Storm => 0.3,
            WeatherType::Snow => 0.6,
            WeatherType::Fog => 0.4,
            WeatherType::Sandstorm => 0.2,
        }
    }

    /// Get suggested biome-appropriate weather for the given biome
    pub fn get_biome_appropriate_weather(biome: astraweave_terrain::BiomeType) -> Vec<WeatherType> {
        use astraweave_terrain::BiomeType;

        match biome {
            BiomeType::Desert => vec![WeatherType::Clear, WeatherType::Sandstorm],
            BiomeType::Tundra => vec![WeatherType::Snow, WeatherType::Clear, WeatherType::Fog],
            BiomeType::Forest => vec![WeatherType::Clear, WeatherType::Rain, WeatherType::Fog],
            BiomeType::Swamp => vec![WeatherType::Fog, WeatherType::Rain, WeatherType::Cloudy],
            BiomeType::Mountain => vec![WeatherType::Clear, WeatherType::Snow, WeatherType::Storm],
            BiomeType::Grassland => {
                vec![WeatherType::Clear, WeatherType::Rain, WeatherType::Cloudy]
            }
            BiomeType::Beach => vec![WeatherType::Clear, WeatherType::Storm, WeatherType::Fog],
            BiomeType::River => vec![WeatherType::Clear, WeatherType::Rain, WeatherType::Fog],
            // Fallback for future biome types
            _ => vec![WeatherType::Clear, WeatherType::Rain],
        }
    }

    // Private helper methods

    fn update_weather_parameters(&mut self) {
        let t = self.transition_progress;

        // Calculate target parameters for current weather
        let (target_rain, target_snow, target_fog, target_wind) = match self.target_weather {
            WeatherType::Clear => (0.0, 0.0, 0.0, 0.1),
            WeatherType::Cloudy => (0.0, 0.0, 0.1, 0.2),
            WeatherType::Rain => (0.7, 0.0, 0.2, 0.4),
            WeatherType::Storm => (1.0, 0.0, 0.3, 0.8),
            WeatherType::Snow => (0.0, 0.8, 0.1, 0.3),
            WeatherType::Fog => (0.0, 0.0, 0.9, 0.1),
            WeatherType::Sandstorm => (0.0, 0.0, 0.4, 1.0),
        };

        // Calculate current parameters for current weather (if transitioning)
        let (current_rain, current_snow, current_fog, current_wind) = if t < 1.0 {
            match self.current_weather {
                WeatherType::Clear => (0.0, 0.0, 0.0, 0.1),
                WeatherType::Cloudy => (0.0, 0.0, 0.1, 0.2),
                WeatherType::Rain => (0.7, 0.0, 0.2, 0.4),
                WeatherType::Storm => (1.0, 0.0, 0.3, 0.8),
                WeatherType::Snow => (0.0, 0.8, 0.1, 0.3),
                WeatherType::Fog => (0.0, 0.0, 0.9, 0.1),
                WeatherType::Sandstorm => (0.0, 0.0, 0.4, 1.0),
            }
        } else {
            (target_rain, target_snow, target_fog, target_wind)
        };

        // Interpolate between current and target
        self.rain_intensity = current_rain + (target_rain - current_rain) * t;
        self.snow_intensity = current_snow + (target_snow - current_snow) * t;
        self.fog_density = current_fog + (target_fog - current_fog) * t;
        self.wind_strength = current_wind + (target_wind - current_wind) * t;
    }
}

/// Weather particle system for rendering precipitation
pub struct WeatherParticles {
    rain_particles: Vec<WeatherParticle>,
    snow_particles: Vec<WeatherParticle>,
    max_particles: usize,
    particle_area: f32, // Area around camera to spawn particles
}

#[derive(Debug, Clone)]
pub struct WeatherParticle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
}

impl WeatherParticles {
    /// Create a new weather particle system
    pub fn new(max_particles: usize, area: f32) -> Self {
        Self {
            rain_particles: Vec::new(),
            snow_particles: Vec::new(),
            max_particles,
            particle_area: area,
        }
    }

    /// Update particle system
    pub fn update(&mut self, delta_time: f32, camera_pos: Vec3, weather: &WeatherSystem) {
        // Update rain particles
        if weather.is_raining() {
            self.update_rain_particles(delta_time, camera_pos, weather);
        } else {
            self.rain_particles.clear();
        }

        // Update snow particles
        if weather.is_snowing() {
            self.update_snow_particles(delta_time, camera_pos, weather);
        } else {
            self.snow_particles.clear();
        }
    }

    /// Get current rain particles for rendering
    pub fn rain_particles(&self) -> &[WeatherParticle] {
        &self.rain_particles
    }

    /// Get current snow particles for rendering
    pub fn snow_particles(&self) -> &[WeatherParticle] {
        &self.snow_particles
    }

    // Private helper methods

    fn update_rain_particles(
        &mut self,
        delta_time: f32,
        camera_pos: Vec3,
        weather: &WeatherSystem,
    ) {
        let target_count = (self.max_particles as f32 * weather.get_rain_intensity()) as usize;

        // Spawn new particles if needed
        while self.rain_particles.len() < target_count {
            let offset = Vec3::new(
                (rand::random::<f32>() - 0.5) * self.particle_area,
                rand::random::<f32>() * 50.0 + 20.0, // Spawn above camera
                (rand::random::<f32>() - 0.5) * self.particle_area,
            );

            let wind = weather.get_wind_direction() * weather.get_wind_strength();

            self.rain_particles.push(WeatherParticle {
                position: camera_pos + offset,
                velocity: vec3(wind.x * 2.0, -15.0, wind.z * 2.0), // Fast downward motion
                life: 0.0,
                max_life: 3.0,
                size: 0.1,
            });
        }

        // Update existing particles
        self.rain_particles.retain_mut(|particle| {
            particle.life += delta_time;
            particle.position += particle.velocity * delta_time;

            // Remove particles that are too old or too far from camera
            particle.life < particle.max_life
                && (particle.position - camera_pos).length() < self.particle_area * 0.6
        });
    }

    fn update_snow_particles(
        &mut self,
        delta_time: f32,
        camera_pos: Vec3,
        weather: &WeatherSystem,
    ) {
        let target_count = (self.max_particles as f32 * weather.get_snow_intensity()) as usize;

        // Spawn new particles if needed
        while self.snow_particles.len() < target_count {
            let offset = Vec3::new(
                (rand::random::<f32>() - 0.5) * self.particle_area,
                rand::random::<f32>() * 30.0 + 15.0, // Spawn above camera
                (rand::random::<f32>() - 0.5) * self.particle_area,
            );

            let wind = weather.get_wind_direction() * weather.get_wind_strength();

            self.snow_particles.push(WeatherParticle {
                position: camera_pos + offset,
                velocity: vec3(wind.x, -2.0, wind.z), // Slower, more drifting motion
                life: 0.0,
                max_life: 10.0, // Snow lasts longer
                size: 0.2,
            });
        }

        // Update existing particles
        self.snow_particles.retain_mut(|particle| {
            particle.life += delta_time;
            particle.position += particle.velocity * delta_time;

            // Add some randomness to snow movement
            particle.velocity.x += (rand::random::<f32>() - 0.5) * 0.1;
            particle.velocity.z += (rand::random::<f32>() - 0.5) * 0.1;

            // Remove particles that are too old or too far from camera
            particle.life < particle.max_life
                && (particle.position - camera_pos).length() < self.particle_area * 0.6
        });
    }
}

const EQUIRECT_SKY_SHADER_SOURCE: &str = r#"
struct SkyUniforms {
    view_proj: mat4x4<f32>,
    sun_position: vec4<f32>,
    moon_position: vec4<f32>,
    top_color: vec4<f32>,
    horizon_color: vec4<f32>,
    time_of_day: f32,
    cloud_coverage: f32,
    cloud_speed: f32,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> uniforms: SkyUniforms;
@group(0) @binding(1) var sky_tex: texture_2d<f32>;
@group(0) @binding(2) var sky_samp: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Remove translation from view matrix to make skybox always centered on camera
    var view_no_translation = uniforms.view_proj;
    view_no_translation[3][0] = 0.0;
    view_no_translation[3][1] = 0.0;
    view_no_translation[3][2] = 0.0;
    
    out.clip_position = view_no_translation * vec4<f32>(input.position, 1.0);
    out.clip_position.z = out.clip_position.w; // Force max depth
    out.world_position = input.position;
    return out;
}

fn dir_to_equirect_uv(dir: vec3<f32>) -> vec2<f32> {
    let n = normalize(dir);
    let phi = atan2(n.z, n.x);
    let theta = acos(clamp(n.y, -1.0, 1.0));
    let u = (phi / 6.2831853 + 0.5);
    let v = theta / 3.14159265;
    return vec2<f32>(u, v);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(input.world_position);
    let uv = dir_to_equirect_uv(dir);
    var color = textureSample(sky_tex, sky_samp, uv);
    // Fade to black below/near horizon to eliminate the hard geometric
    // edge of the sky sphere. The ground fill plane renders on top anyway.
    let horizon_fade = smoothstep(-0.05, 0.1, dir.y);
    color = vec4(color.rgb * horizon_fade, color.a);
    return color;
}
"#;
