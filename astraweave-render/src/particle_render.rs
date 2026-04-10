//! Billboard Particle Render Pipeline — instanced quads with blending.
//!
//! Renders particles as camera-facing billboard quads using instanced drawing.
//! Supports additive and alpha blending modes, soft-particle depth fading,
//! and optional texture sampling.
//!
//! Particles are drawn in sorted order (via `ParticleSortPass`) for correct
//! alpha transparency.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Camera uniforms for billboard orientation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ParticleCameraUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad0: f32,
    pub camera_right: [f32; 3],
    pub _pad1: f32,
    pub camera_up: [f32; 3],
    pub near_plane: f32,
    pub inv_resolution: [f32; 2],
    pub soft_depth_range: f32,
    pub _pad2: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Blend mode for particle rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleBlendMode {
    /// Additive: src + dst (good for fire, sparks, magic).
    Additive,
    /// Alpha: src*alpha + dst*(1-alpha) (good for smoke, dust).
    AlphaBlend,
    /// Premultiplied alpha: src + dst*(1-src.a).
    PremultipliedAlpha,
}

impl ParticleBlendMode {
    fn to_blend_state(self) -> wgpu::BlendState {
        match self {
            ParticleBlendMode::Additive => wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
            ParticleBlendMode::AlphaBlend => wgpu::BlendState::ALPHA_BLENDING,
            ParticleBlendMode::PremultipliedAlpha => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        }
    }
}

/// Configuration for the particle renderer.
#[derive(Debug, Clone)]
pub struct ParticleRenderConfig {
    pub blend_mode: ParticleBlendMode,
    /// Range in NDC depth for soft-particle fading.
    pub soft_depth_range: f32,
    /// Write to depth buffer (typically false for particles).
    pub depth_write: bool,
}

impl Default for ParticleRenderConfig {
    fn default() -> Self {
        Self {
            blend_mode: ParticleBlendMode::Additive,
            soft_depth_range: 0.5,
            depth_write: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Particle Render Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for billboard particle rendering.
pub struct ParticleRenderPass {
    config: ParticleRenderConfig,
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    camera_buf: wgpu::Buffer,
    /// Fallback 1x1 white texture when no particle texture is bound.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    default_texture: wgpu::Texture,
    default_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl ParticleRenderPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        config: ParticleRenderConfig,
    ) -> Self {
        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("particle_camera"),
            contents: bytemuck::bytes_of(&ParticleCameraUniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 1x1 white fallback texture
        let default_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("particle_default_tex"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &default_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let default_view = default_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("particle_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_render_bgl"),
            entries: &[
                // 0: camera
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: particles (storage, read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 2: sort indices (storage, read)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: depth texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 4: particle texture
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 5: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("particle_render_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/particles/render.wgsl").into(),
            ),
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_render_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle_render_pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers — all data from storage buffers
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(config.blend_mode.to_blend_state()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // Billboards face both ways
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: config.depth_write,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            config,
            pipeline,
            bgl,
            camera_buf,
            default_texture,
            default_view,
            sampler,
        }
    }

    pub fn config(&self) -> &ParticleRenderConfig {
        &self.config
    }

    /// Update camera uniforms for this frame.
    pub fn update_camera(
        &self,
        queue: &wgpu::Queue,
        view: Mat4,
        proj: Mat4,
        camera_pos: Vec3,
        width: u32,
        height: u32,
    ) {
        let view_proj = proj * view;
        let inv_view = view.inverse();
        let right = Vec3::new(inv_view.col(0).x, inv_view.col(0).y, inv_view.col(0).z);
        let up = Vec3::new(inv_view.col(1).x, inv_view.col(1).y, inv_view.col(1).z);

        let uniforms = ParticleCameraUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            camera_pos: camera_pos.to_array(),
            _pad0: 0.0,
            camera_right: right.to_array(),
            _pad1: 0.0,
            camera_up: up.to_array(),
            near_plane: 0.1,
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            soft_depth_range: self.config.soft_depth_range,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Render particles into the given render pass.
    /// `particle_count` is the actual number of live particles (not max capacity).
    #[allow(clippy::too_many_arguments)]
    pub fn render<'a>(
        &'a self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'a>,
        particle_buffer: &'a wgpu::Buffer,
        sort_entries_buffer: &'a wgpu::Buffer,
        depth_view: &'a wgpu::TextureView,
        particle_count: u32,
        texture_view: Option<&'a wgpu::TextureView>,
    ) {
        if particle_count == 0 {
            return;
        }

        let tex_view = texture_view.unwrap_or(&self.default_view);

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_render_bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: sort_entries_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bg, &[]);
        // 6 vertices per quad (2 triangles), instanced by particle count
        pass.draw(0..6, 0..particle_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_uniforms_size() {
        // mat4(64) + vec3+pad(16) + vec3+pad(16) + vec3+f32(16) + 4×f32(16) = 128
        assert_eq!(std::mem::size_of::<ParticleCameraUniforms>(), 128);
    }

    #[test]
    fn default_config() {
        let c = ParticleRenderConfig::default();
        assert_eq!(c.blend_mode, ParticleBlendMode::Additive);
        assert!(!c.depth_write);
        assert!(c.soft_depth_range > 0.0);
    }

    #[test]
    fn blend_modes() {
        let additive = ParticleBlendMode::Additive.to_blend_state();
        assert_eq!(additive.color.dst_factor, wgpu::BlendFactor::One);

        let alpha = ParticleBlendMode::AlphaBlend.to_blend_state();
        assert_eq!(alpha.color.dst_factor, wgpu::BlendFactor::OneMinusSrcAlpha);
    }

    #[test]
    fn particle_render_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = ParticleRenderPass::new(
            &device,
            &queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Depth32Float,
            ParticleRenderConfig::default(),
        );
        assert_eq!(pass.config().blend_mode, ParticleBlendMode::Additive);
    }
}
