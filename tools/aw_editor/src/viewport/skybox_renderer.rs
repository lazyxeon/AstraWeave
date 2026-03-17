//! Skybox Renderer
//!
//! Renders procedural gradient skybox or HDRI environment map.
//! Uses fullscreen quad rendered at infinite distance.

#![allow(dead_code)]

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use std::path::Path;

use super::camera::OrbitCamera;

/// Skybox renderer — supports procedural gradient and HDRI equirectangular maps
pub struct SkyboxRenderer {
    /// Procedural gradient pipeline
    procedural_pipeline: wgpu::RenderPipeline,

    /// HDRI pipeline (uses texture sampling)
    hdri_pipeline: wgpu::RenderPipeline,

    /// Bind group layout for procedural mode (uniforms only)
    procedural_bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group layout for HDRI mode (uniforms + texture + sampler)
    hdri_bind_group_layout: wgpu::BindGroupLayout,

    /// Bind group for procedural mode
    procedural_bind_group: wgpu::BindGroup,

    /// Bind group for HDRI mode (created when HDRI is loaded)
    hdri_bind_group: Option<wgpu::BindGroup>,

    /// Uniform buffer (camera + colors)
    uniform_buffer: wgpu::Buffer,

    /// Whether HDRI mode is active
    hdri_active: bool,

    /// Configurable sky colors (overridden by presets/time-of-day)
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ground_color: [f32; 4],
}

impl SkyboxRenderer {
    /// Create new skybox renderer
    pub fn new(device: &wgpu::Device) -> Result<Self> {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skybox Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/skybox.wgsl").into()),
        });

        // -- Procedural bind group layout (uniforms only) --
        let procedural_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Skybox Procedural Bind Group Layout"),
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

        // -- HDRI bind group layout (uniforms + texture + sampler) --
        let hdri_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Skybox HDRI Bind Group Layout"),
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

        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        };

        let multisample = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let targets = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];

        // -- Procedural pipeline --
        let procedural_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Procedural Pipeline Layout"),
            bind_group_layouts: &[&procedural_bind_group_layout],
            push_constant_ranges: &[],
        });

        let procedural_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Skybox Procedural Pipeline"),
                layout: Some(&procedural_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive,
                depth_stencil: Some(depth_stencil.clone()),
                multisample,
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &targets,
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });

        // -- HDRI pipeline --
        let hdri_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox HDRI Pipeline Layout"),
            bind_group_layouts: &[&hdri_bind_group_layout],
            push_constant_ranges: &[],
        });

        let hdri_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox HDRI Pipeline"),
            layout: Some(&hdri_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive,
            depth_stencil: Some(depth_stencil),
            multisample,
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_hdri"),
                targets: &targets,
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skybox Uniform Buffer"),
            size: std::mem::size_of::<SkyboxUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create procedural bind group
        let procedural_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skybox Procedural Bind Group"),
            layout: &procedural_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            procedural_pipeline,
            hdri_pipeline,
            procedural_bind_group_layout,
            hdri_bind_group_layout,
            procedural_bind_group,
            hdri_bind_group: None,
            uniform_buffer,
            hdri_active: false,
            sky_top: [0.1, 0.3, 0.8, 1.0],
            sky_horizon: [0.5, 0.7, 0.95, 1.0],
            ground_color: [0.2, 0.15, 0.1, 1.0],
        })
    }

    /// Load an HDRI image file and apply it as the skybox background.
    ///
    /// Supports .hdr (Radiance), .exr, .png, .jpg formats.
    /// The image is treated as an equirectangular projection.
    pub fn load_hdri(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<()> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read HDRI file: {}", path.display()))?;

        // Decode to RGBA8 pixel data
        let img = image::load_from_memory(&data)
            .with_context(|| format!("Failed to decode image: {}", path.display()))?;

        // For HDR images (Rgb32F), apply Reinhard tonemapping
        let rgba_img = match &img {
            image::DynamicImage::ImageRgb32F(rgb32f) => {
                let (w, h) = (rgb32f.width(), rgb32f.height());
                let mut rgba = image::RgbaImage::new(w, h);
                for (x, y, px) in rgb32f.enumerate_pixels() {
                    let r = px[0] / (1.0 + px[0]);
                    let g = px[1] / (1.0 + px[1]);
                    let b = px[2] / (1.0 + px[2]);
                    rgba.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            (r.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8,
                            (g.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8,
                            (b.powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8,
                            255,
                        ]),
                    );
                }
                rgba
            }
            _ => img.to_rgba8(),
        };

        let width = rgba_img.width();
        let height = rgba_img.height();
        let rgba_data = rgba_img.into_raw();

        // Create wgpu texture
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HDRI Skybox Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("HDRI Skybox Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create HDRI bind group
        let hdri_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skybox HDRI Bind Group"),
            layout: &self.hdri_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.hdri_bind_group = Some(hdri_bind_group);
        self.hdri_active = true;

        tracing::info!("HDRI skybox loaded: {}x{} from {}", width, height, path.display());

        Ok(())
    }

    /// Remove the HDRI and revert to procedural gradient
    pub fn clear_hdri(&mut self) {
        self.hdri_bind_group = None;
        self.hdri_active = false;
    }

    /// Set the procedural sky gradient colors
    pub fn set_sky_colors(
        &mut self,
        sky_top: [f32; 4],
        sky_horizon: [f32; 4],
        ground_color: [f32; 4],
    ) {
        self.sky_top = sky_top;
        self.sky_horizon = sky_horizon;
        self.ground_color = ground_color;
    }

    /// Render skybox
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        camera: &OrbitCamera,
        queue: &wgpu::Queue,
    ) -> Result<()> {
        // Update uniforms
        let view_proj = camera.view_projection_matrix();
        let inv_view_proj = view_proj.inverse();

        let uniforms = SkyboxUniforms {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            camera_pos: camera.position().to_array(),
            _padding: 0.0,
            sky_top: self.sky_top,
            sky_horizon: self.sky_horizon,
            ground_color: self.ground_color,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Skybox Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.2,
                        g: 0.15,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        if self.hdri_active {
            if let Some(ref bg) = self.hdri_bind_group {
                pass.set_pipeline(&self.hdri_pipeline);
                pass.set_bind_group(0, bg, &[]);
            } else {
                pass.set_pipeline(&self.procedural_pipeline);
                pass.set_bind_group(0, &self.procedural_bind_group, &[]);
            }
        } else {
            pass.set_pipeline(&self.procedural_pipeline);
            pass.set_bind_group(0, &self.procedural_bind_group, &[]);
        }

        pass.draw(0..6, 0..1);

        Ok(())
    }
}

/// Skybox shader uniforms
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SkyboxUniforms {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 3],
    _padding: f32,
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    ground_color: [f32; 4],
}
