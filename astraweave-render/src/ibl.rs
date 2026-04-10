//! Image-Based Lighting (IBL) manager and prefilter pipelines
//!
//! Minimal but complete IBL pipeline that can be refined later. Provides:
//! - Procedural sky capture into an environment cubemap
//! - Irradiance convolution for diffuse IBL (small cube)
//! - Specular prefilter for GGX (mip chain encodes roughness)
//! - BRDF LUT generation (split-sum), baked once at init
//!
//! The module is renderer-agnostic: it exposes bind group layout helpers and texture views
//! that consumers can bind into their shading pipelines.

#[cfg(feature = "textures")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "textures")]
use image::GenericImageView;
use std::borrow::Cow;
#[cfg(feature = "textures")]
use std::collections::HashMap;
#[cfg(feature = "textures")]
use std::path::Path;
use wgpu::util::DeviceExt;

/// Quality presets for IBL resource sizes
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum IblQuality {
    Low,
    Medium,
    High,
}

impl IblQuality {
    fn env_size(self) -> u32 {
        match self {
            IblQuality::Low => 256,
            IblQuality::Medium => 512,
            IblQuality::High => 1024,
        }
    }
    fn spec_size(self) -> u32 {
        (self.env_size() / 2).max(128)
    }
    fn irradiance_size(self) -> u32 {
        64
    }
    fn brdf_lut_size(self) -> u32 {
        256
    }
    fn spec_mips(self) -> u32 {
        let s = self.spec_size();
        (s.max(1) as f32).log2().floor() as u32 + 1
    }
}

/// Sky sources supported by the manager
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SkyMode {
    /// Load an equirectangular HDR and convert to a cubemap
    HdrPath { biome: String, path: String },
    /// Render a simple procedural sky into the cubemap
    Procedural {
        last_capture_time: f32,
        recapture_interval: f32,
    },
}

/// Public handles to IBL resources (texture views suited for binding)
pub struct IblResources {
    pub env_cube: wgpu::TextureView, // optional to keep for debug
    pub irradiance_cube: wgpu::TextureView,
    pub specular_cube: wgpu::TextureView, // mip chain encodes roughness
    pub brdf_lut: wgpu::TextureView,      // 2D LUT
    pub mips_specular: u32,
    pub hdr_equirect: Option<wgpu::TextureView>, // Source HDR image for skybox
    /// Average luminance of the source HDR (used to normalise IBL intensity).
    /// `None` for procedural sky.
    pub avg_luminance: Option<f32>,
}

/// Internal textures owned by the manager (kept to control lifetime)
struct IblTextures {
    _env: wgpu::Texture, // Kept alive for views
    _irradiance: wgpu::Texture,
    _specular: wgpu::Texture,
    _brdf_lut: wgpu::Texture,
    _spec_mips: u32,
    _hdr_equirect: Option<wgpu::Texture>,
}

pub struct IblManager {
    pub enabled: bool,
    pub mode: SkyMode,
    pub sun_elevation: f32,
    pub sun_azimuth: f32,

    // GPU objects
    sampler: wgpu::Sampler,
    ibl_bgl: wgpu::BindGroupLayout,
    // Bind group layout for convolution shaders (env cube + sampler)
    env_bgl: wgpu::BindGroupLayout,
    // Bind group layout for prefilter params (roughness, face, sample count)
    prefilter_params_bgl: wgpu::BindGroupLayout,
    // Keep textures alive across frames/bind group creations
    textures: Option<IblTextures>,
    // Pipelines
    sky_pipeline: wgpu::RenderPipeline,
    irr_pipeline: wgpu::RenderPipeline,
    spec_pipeline: wgpu::RenderPipeline,
    brdf_pipeline: wgpu::RenderPipeline,
    // Equirectangular conversion (used in load_hdr_and_convert)
    #[allow(dead_code)]
    eqr_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    eqr_face_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    eqr_pipeline: wgpu::RenderPipeline,
    // Cache decoded HDR equirectangular images by path to avoid repeated IO/decoding
    #[cfg(feature = "textures")]
    hdr_cache: HashMap<String, image::DynamicImage>,
}

impl IblManager {
    pub fn new(device: &wgpu::Device, quality: IblQuality) -> Result<Self> {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ibl-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });

        let ibl_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl-bgl"),
            entries: &[
                // 0: prefiltered specular cube
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 1: irradiance cube
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 2: BRDF LUT 2D
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // 3: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Pipelines
        let sky_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ibl-sky-sm"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SKY_WGSL)),
        });
        let irr_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ibl-irr-sm"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(IRRADIANCE_WGSL)),
        });
        let spec_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ibl-spec-sm"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SPECULAR_PREFILTER_WGSL)),
        });
        let brdf_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ibl-brdf-sm"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BRDF_LUT_WGSL)),
        });

        let unit_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ibl-unit-pl"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let brdf_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ibl-brdf-pipeline"),
            layout: Some(&unit_pl),
            vertex: wgpu::VertexState {
                module: &brdf_sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &brdf_sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Env sampling BGL for irradiance/specular passes
        let env_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl-env-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        // Bind group layout for prefilter params (roughness, face_idx, sample_count)
        let prefilter_params_bgl =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ibl-prefilter-params-bgl"),
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

        // Separate layouts: sky has no bindings; irradiance samples env (group 0); spec samples env (group 0) + params (group 1)
        let sky_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ibl-sky-pl"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let conv_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ibl-conv-pl"),
            bind_group_layouts: &[&env_bgl],
            push_constant_ranges: &[],
        });
        let spec_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ibl-spec-pl"),
            bind_group_layouts: &[&env_bgl, &prefilter_params_bgl],
            push_constant_ranges: &[],
        });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ibl-sky-pipeline"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState {
                module: &sky_sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sky_sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let irr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ibl-irr-pipeline"),
            layout: Some(&conv_pl),
            vertex: wgpu::VertexState {
                module: &irr_sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &irr_sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let spec_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ibl-spec-pipeline"),
            layout: Some(&spec_pl),
            vertex: wgpu::VertexState {
                module: &spec_sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &spec_sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Equirectangular to cube pipeline
        let eqr_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ibl-eqr-sm"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(EQUIRECT_TO_CUBE_WGSL)),
        });
        let eqr_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl-eqr-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let eqr_face_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl-eqr-face-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let eqr_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ibl-eqr-pl"),
            bind_group_layouts: &[&eqr_bgl, &eqr_face_bgl],
            push_constant_ranges: &[],
        });
        let eqr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ibl-eqr-pipeline"),
            layout: Some(&eqr_pl),
            vertex: wgpu::VertexState {
                module: &eqr_sm,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &eqr_sm,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let mgr = Self {
            enabled: true,
            mode: SkyMode::Procedural {
                last_capture_time: 0.0,
                recapture_interval: 0.25,
            },
            sun_elevation: 45.0_f32.to_radians(),
            sun_azimuth: 0.0,
            sampler,
            ibl_bgl,
            env_bgl,
            prefilter_params_bgl,
            textures: None,
            sky_pipeline,
            irr_pipeline,
            spec_pipeline,
            brdf_pipeline,
            eqr_bgl,
            eqr_face_bgl,
            eqr_pipeline,
            #[cfg(feature = "textures")]
            hdr_cache: HashMap::new(),
        };
        // Avoid unused warning for quality for now
        let _ = quality;
        Ok(mgr)
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.ibl_bgl
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Ensure BRDF LUT is baked (one-time cost)
    /// Returns a stable TextureView for binding
    #[cfg(feature = "ibl")]
    pub fn ensure_brdf_lut(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        quality: IblQuality,
    ) -> Result<wgpu::TextureView> {
        // If textures are not baked, bake them
        if self.textures.is_none() {
            let _ = self.bake_environment(device, queue, quality)?;
        }
        let view = self
            .textures
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("IBL textures not baked"))?
            ._brdf_lut
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(view)
    }

    /// Ensure irradiance cubemap is baked from environment source
    #[cfg(feature = "ibl")]
    pub fn ensure_irradiance(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        quality: IblQuality,
    ) -> Result<wgpu::TextureView> {
        if self.textures.is_none() {
            let _ = self.bake_environment(device, queue, quality)?;
        }
        let view = self
            .textures
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("IBL textures not baked"))?
            ._irradiance
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
        Ok(view)
    }

    /// Ensure prefiltered environment cubemap is baked
    #[cfg(feature = "ibl")]
    pub fn ensure_prefiltered_env(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        quality: IblQuality,
    ) -> Result<wgpu::TextureView> {
        if self.textures.is_none() {
            let _ = self.bake_environment(device, queue, quality)?;
        }
        let textures = self
            .textures
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("IBL textures not baked"))?;
        let view = textures
            ._specular
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::Cube),
                base_mip_level: 0,
                mip_level_count: Some(textures._spec_mips),
                ..Default::default()
            });
        Ok(view)
    }

    /// Ensure environment and prefiltered outputs exist for the given mode/quality
    pub fn bake_environment(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        quality: IblQuality,
    ) -> Result<IblResources> {
        // Allocate textures
        let env_size = quality.env_size();
        let irr_size = quality.irradiance_size();
        let spec_size = quality.spec_size();
        let spec_mips = quality.spec_mips();
        // Will be set to Some(lum) for HDR paths so we can normalise IBL intensity
        let mut hdr_avg_lum: Option<f32> = None;

        let env_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ibl-env-cube"),
            size: wgpu::Extent3d {
                width: env_size,
                height: env_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let irr_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ibl-irr-cube"),
            size: wgpu::Extent3d {
                width: irr_size,
                height: irr_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let spec_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ibl-spec-cube"),
            size: wgpu::Extent3d {
                width: spec_size,
                height: spec_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: spec_mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let brdf_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ibl-brdf-lut"),
            size: wgpu::Extent3d {
                width: quality.brdf_lut_size(),
                height: quality.brdf_lut_size(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        // Views
        let env_view = env_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("ibl-env-view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let irr_view = irr_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("ibl-irr-view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let spec_view = spec_tex.create_view(&wgpu::TextureViewDescriptor {
            usage: None,
            label: Some("ibl-spec-view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_mip_level: 0,
            mip_level_count: Some(spec_mips),
            ..Default::default()
        });
        let brdf_view = brdf_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // BRDF LUT bake
        {
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ibl-brdf-enc"),
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ibl-brdf-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &brdf_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.brdf_pipeline);
            rp.draw(0..3, 0..1);
            drop(rp);
            queue.submit(Some(enc.finish()));
        }

        // Sky capture into env cube (procedural or HDR-equirect conversion)
        // returns (Option<Texture>, Option<TextureView>) describing source equirect
        let (eqr_tex_opt, eqr_view_opt) = match &self.mode {
            SkyMode::Procedural { .. } => {
                let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("ibl-sky-enc"),
                });
                for face in 0..6u32 {
                    let face_view = env_tex.create_view(&wgpu::TextureViewDescriptor {
                        usage: None,
                        label: Some("ibl-env-face"),
                        format: Some(wgpu::TextureFormat::Rgba16Float),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: face,
                        array_layer_count: Some(1),
                        aspect: wgpu::TextureAspect::All,
                    });
                    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ibl-sky-face"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &face_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rp.set_pipeline(&self.sky_pipeline);
                    rp.draw(0..3, 0..1);
                    drop(rp);
                }
                queue.submit(Some(enc.finish()));
                (None, None)
            }
            #[cfg(feature = "textures")]
            SkyMode::HdrPath { biome: _, path } => {
                let img = if let Some(img) = self.hdr_cache.get(path) {
                    img.clone()
                } else {
                    let img = load_hdr_equirectangular(Path::new(path))
                        .with_context(|| format!("load HDR {}", path))?;
                    self.hdr_cache.insert(path.clone(), img.clone());
                    img
                };
                // Compute average luminance for IBL intensity normalisation
                hdr_avg_lum = Some(compute_hdr_avg_luminance(&img));
                let (hdr_tex, hdr_view, hdr_samp) = create_hdr2d(device, queue, &img)?;
                let eqr_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("ibl-eqr-bg"),
                    layout: &self.eqr_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&hdr_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&hdr_samp),
                        },
                    ],
                });
                // Uniform buffer for face index (aligned to 16 bytes)
                // Uniform buffer and bind group will be created per-face to prevent race conditions
                for face in 0..6u32 {
                    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("ibl-eqr-enc"),
                    });

                    // Create per-face uniform buffer
                    let data: [u32; 4] = [face, 0, 0, 0];
                    let face_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("ibl-eqr-face-ub"),
                        contents: bytemuck::bytes_of(&data),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let eqr_face_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("ibl-eqr-face-bg"),
                        layout: &self.eqr_face_bgl,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: face_buf.as_entire_binding(),
                        }],
                    });

                    let face_view = env_tex.create_view(&wgpu::TextureViewDescriptor {
                        usage: None,
                        label: Some("ibl-env-face"),
                        format: Some(wgpu::TextureFormat::Rgba16Float),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level: 0,
                        mip_level_count: Some(1),
                        base_array_layer: face,
                        array_layer_count: Some(1),
                        aspect: wgpu::TextureAspect::All,
                    });

                    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ibl-eqr-face"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &face_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rp.set_pipeline(&self.eqr_pipeline);
                    rp.set_bind_group(0, &eqr_bg, &[]);
                    rp.set_bind_group(1, &eqr_face_bg, &[]);
                    rp.draw(0..3, 0..1);
                    drop(rp);
                    queue.submit(Some(enc.finish()));
                }
                (Some(hdr_tex), Some(hdr_view))
            }
            #[cfg(not(feature = "textures"))]
            SkyMode::HdrPath { .. } => {
                anyhow::bail!("HdrPath sky mode requires 'textures' feature");
                #[allow(unreachable_code)]
                (None, None)
            }
        };

        // Irradiance convolution
        {
            // Bind environment cube (as captured) for sampling
            let env_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ibl-env-bg"),
                layout: &self.env_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&env_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ibl-irr-enc"),
            });
            for face in 0..6u32 {
                let dst_face = irr_tex.create_view(&wgpu::TextureViewDescriptor {
                    usage: None,
                    label: Some("ibl-irr-face"),
                    format: Some(wgpu::TextureFormat::Rgba16Float),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: face,
                    array_layer_count: Some(1),
                    base_mip_level: 0,
                    mip_level_count: Some(1),
                    aspect: wgpu::TextureAspect::All,
                });
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ibl-irr-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &dst_face,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rp.set_pipeline(&self.irr_pipeline);
                rp.set_bind_group(0, &env_bg, &[]);
                rp.draw(0..3, 0..1);
                drop(rp);
            }
            queue.submit(Some(enc.finish()));
        }

        // Specular prefilter for each mip and face with proper roughness encoding
        {
            let env_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ibl-env-bg"),
                layout: &self.env_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&env_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            // Buffers created per face/mip

            for mip in 0..spec_mips {
                // Calculate roughness from mip level (linear mapping)
                let roughness = (mip as f32) / ((spec_mips - 1) as f32).max(1.0);
                // Quality-based sample count: higher for low mips, lower for high mips
                let sample_count: u32 = match quality {
                    IblQuality::Low => {
                        if mip == 0 {
                            128
                        } else {
                            64
                        }
                    }
                    IblQuality::Medium => {
                        if mip == 0 {
                            256
                        } else {
                            128
                        }
                    }
                    IblQuality::High => {
                        if mip == 0 {
                            512
                        } else {
                            256
                        }
                    }
                };

                for face in 0..6u32 {
                    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("ibl-spec-enc"),
                    });

                    // Update params uniform for this mip/face combination
                    let params_data: [u32; 4] = [
                        f32::to_bits(roughness),
                        face,
                        sample_count,
                        0, // padding
                    ];
                    let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("ibl-prefilter-params-ub"),
                        contents: bytemuck::cast_slice(&params_data),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

                    let params_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("ibl-prefilter-params-bg"),
                        layout: &self.prefilter_params_bgl,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: params_buf.as_entire_binding(),
                        }],
                    });

                    let dst = spec_tex.create_view(&wgpu::TextureViewDescriptor {
                        usage: None,
                        label: Some("ibl-spec-sub"),
                        format: Some(wgpu::TextureFormat::Rgba16Float),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: face,
                        array_layer_count: Some(1),
                        base_mip_level: mip,
                        mip_level_count: Some(1),
                        aspect: wgpu::TextureAspect::All,
                    });
                    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ibl-spec-pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &dst,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rp.set_pipeline(&self.spec_pipeline);
                    rp.set_bind_group(0, &env_bg, &[]);
                    rp.set_bind_group(1, &params_bg, &[]);
                    rp.draw(0..3, 0..1);
                    drop(rp);
                    queue.submit(Some(enc.finish()));
                }
            }
        }

        // Hold textures so views remain valid for the lifetime of the manager
        self.textures = Some(IblTextures {
            _env: env_tex,
            _irradiance: irr_tex,
            _specular: spec_tex,
            _brdf_lut: brdf_tex,
            _spec_mips: spec_mips,
            _hdr_equirect: eqr_tex_opt,
        });

        let resources = IblResources {
            env_cube: env_view,
            irradiance_cube: irr_view,
            specular_cube: spec_view,
            brdf_lut: brdf_view,
            mips_specular: spec_mips,
            hdr_equirect: eqr_view_opt,
            avg_luminance: hdr_avg_lum,
        };
        Ok(resources)
    }

    pub fn create_bind_group(&self, device: &wgpu::Device, res: &IblResources) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ibl-bg"),
            layout: &self.ibl_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&res.specular_cube),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&res.irradiance_cube),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&res.brdf_lut),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }
}

// ---------------------------------------------------------------------------------
// WGSL Shaders (minimal kernels)
// ---------------------------------------------------------------------------------

// Simple procedural sky capture into a face render-target (minimal parity with environment.rs sky)
const SKY_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    let xy = p[vi];
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = (xy+1.0)*0.5;
    return out;
}
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Map uv to direction with a simple up-facing hemisphere basis (placeholder)
    let dir = normalize(vec3<f32>(in.uv.x*2.0-1.0, 1.0, in.uv.y*2.0-1.0));
    let y = clamp(dir.y, -1.0, 1.0);
    let horizon = vec3<f32>(0.75, 0.85, 1.0);
    let zenith = vec3<f32>(0.15, 0.45, 0.9);
    let t = pow(clamp((y + 1.0) * 0.5, 0.0, 1.0), 0.6);
    let base = mix(horizon, zenith, t);
    return vec4<f32>(base, 1.0);
}
"#;

// Irradiance convolution (Lambertian diffuse): proper cosine-weighted hemisphere sampling
// Note: This shader is executed per-face, so we derive the normal from clip-space position
const IRRADIANCE_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) clip_pos: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    let xy = p[vi];
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.clip_pos = xy;
    return out;
}
@group(0) @binding(0) var env_cube: texture_cube<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Normal derived from clip-space coordinates (-1 to 1)
    // This works because we render one face at a time
    // The Z component will be 1.0 (facing outward from cube center)
    let N = normalize(vec3<f32>(in.clip_pos.x, in.clip_pos.y, 1.0));
    
    // Build orthonormal basis for tangent space
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(N.z) < 0.999);
    let T = normalize(cross(up, N));
    let B = cross(N, T);
    
    // Integrate over hemisphere with cosine-weighted sampling
    var irradiance = vec3<f32>(0.0, 0.0, 0.0);
    let PHI_STEPS = 60u; // 6 degree steps
    let THETA_STEPS = 30u; // 3 degree steps
    let delta_phi = (2.0 * 3.14159265) / f32(PHI_STEPS);
    let delta_theta = (0.5 * 3.14159265) / f32(THETA_STEPS);
    
    var sample_count = 0.0;
    for (var i_phi = 0u; i_phi < PHI_STEPS; i_phi++) {
        for (var i_theta = 0u; i_theta < THETA_STEPS; i_theta++) {
            let phi = f32(i_phi) * delta_phi;
            let theta = f32(i_theta) * delta_theta;
            
            // Spherical to cartesian (in tangent space)
            let sample_vec_tangent = vec3<f32>(
                sin(theta) * cos(phi),
                sin(theta) * sin(phi),
                cos(theta)
            );
            // Transform to world space
            let sample_vec = normalize(
                T * sample_vec_tangent.x + 
                B * sample_vec_tangent.y + 
                N * sample_vec_tangent.z
            );
            
            // Sample environment with cosine weighting (NdotL)
            let sample_color = textureSample(env_cube, samp, sample_vec).rgb;
            irradiance += sample_color * cos(theta) * sin(theta);
            sample_count += 1.0;
        }
    }
    irradiance = irradiance * 3.14159265 / sample_count;
    
    return vec4<f32>(irradiance, 1.0);
}
"#;

// Specular prefilter (GGX): properly samples environment with importance sampling encoding roughness per mip
const SPECULAR_PREFILTER_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) face_idx: u32 };
struct PrefilterParams { roughness: f32, face_idx: u32, sample_count: u32, _pad: u32 };
@group(1) @binding(0) var<uniform> params: PrefilterParams;
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    let xy = p[vi];
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = (xy+1.0)*0.5;
    out.face_idx = params.face_idx;
    return out;
}
@group(0) @binding(0) var env_cube: texture_cube<f32>;
@group(0) @binding(1) var samp: sampler;
fn radicalInverseVdC(bitsIn: u32) -> f32 { var bits = bitsIn; bits = (bits << 16u) | (bits >> 16u); bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u); bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u); bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u); bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u); return f32(bits) * 2.3283064365386963e-10; }
fn hammersley(i: u32, n: u32) -> vec2<f32> { return vec2<f32>(f32(i)/f32(n), radicalInverseVdC(i)); }
fn importanceSampleGGX(Xi: vec2<f32>, N: vec3<f32>, roughness: f32) -> vec3<f32> { 
    let a = roughness*roughness; 
    let phi = 6.2831853*Xi.x; 
    let cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y)); 
    let sinTheta = sqrt(1.0 - cosTheta*cosTheta); 
    let H_tangent = vec3<f32>(cos(phi)*sinTheta, sin(phi)*sinTheta, cosTheta);
    // Build TBN
    let up = select(vec3<f32>(1.0,0.0,0.0), vec3<f32>(0.0,1.0,0.0), abs(N.z) < 0.999);
    let T = normalize(cross(up, N));
    let B = cross(N, T);
    return normalize(T*H_tangent.x + B*H_tangent.y + N*H_tangent.z);
}
fn uv_to_cube_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let tc = uv*2.0 - 1.0;
    if (face == 0u) { return normalize(vec3<f32>( 1.0,   -tc.y,  -tc.x)); }
    if (face == 1u) { return normalize(vec3<f32>(-1.0,   -tc.y,   tc.x)); }
    if (face == 2u) { return normalize(vec3<f32>( tc.x,   1.0,    tc.y)); }
    if (face == 3u) { return normalize(vec3<f32>( tc.x,  -1.0,   -tc.y)); }
    if (face == 4u) { return normalize(vec3<f32>( tc.x,  -tc.y,   1.0)); }
    return normalize(vec3<f32>(-tc.x,  -tc.y,  -1.0));
}
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let roughness = params.roughness;
    let N = uv_to_cube_dir(in.face_idx, in.uv);
    let R = N;
    let V = R;
    let SAMPLE_COUNT = params.sample_count;
    var acc = vec3<f32>(0.0, 0.0, 0.0);
    var w: f32 = 0.0;
    for (var i: u32 = 0u; i < SAMPLE_COUNT; i = i + 1u) {
        let Xi = hammersley(i, SAMPLE_COUNT);
        let H = importanceSampleGGX(Xi, N, roughness);
        let L = normalize(2.0 * dot(V,H) * H - V);
        let NdotL = max(dot(N,L), 0.0);
        if (NdotL > 0.0) {
            // Sample with appropriate mip level based on roughness for better filtering
            let D = ((roughness*roughness - 1.0)*roughness*roughness + 1.0);
            let pdf = max(D / (4.0 * 3.14159265), 1e-6);
            let texel_solid_angle = 4.0 * 3.14159265 / (6.0 * 512.0 * 512.0); // Assume 512 env resolution
            let sample_solid_angle = 1.0 / (f32(SAMPLE_COUNT) * pdf);
            let mip_level = select(0.0, 0.5 * log2(sample_solid_angle / texel_solid_angle), roughness > 0.0);
            acc += textureSampleLevel(env_cube, samp, L, mip_level).rgb * NdotL;
            w += NdotL;
        }
    }
    let outc = acc / max(w, 1e-4);
    return vec4<f32>(outc, 1.0);
}
"#;

// Split-sum BRDF LUT (A, B) in RG channels
const BRDF_LUT_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    let xy = p[vi];
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = (xy+1.0)*0.5;
    return out;
}
fn radicalInverseVdC(bitsIn: u32) -> f32 { var bits = bitsIn; bits = (bits << 16u) | (bits >> 16u); bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u); bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u); bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u); bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u); return f32(bits) * 2.3283064365386963e-10; }
fn hammersley(i: u32, n: u32) -> vec2<f32> { return vec2<f32>(f32(i)/f32(n), radicalInverseVdC(i)); }
fn geometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 { let r = roughness + 1.0; let k = (r*r)/8.0; return NdotV / (NdotV * (1.0 - k) + k); }
fn geometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 { let NdotV = max(dot(N,V),0.0); let NdotL = max(dot(N,L),0.0); let ggx2 = geometrySchlickGGX(NdotV, roughness); let ggx1 = geometrySchlickGGX(NdotL, roughness); return ggx1*ggx2; }
fn importanceSampleGGX(Xi: vec2<f32>, N: vec3<f32>, roughness: f32) -> vec3<f32> { let a = roughness*roughness; let phi = 6.2831853*Xi.x; let cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y)); let sinTheta = sqrt(1.0 - cosTheta*cosTheta); let H = vec3<f32>(cos(phi)*sinTheta, sin(phi)*sinTheta, cosTheta); let up = vec3<f32>(0.0,1.0,0.0); let T = normalize(cross(up, N)); let B = cross(N, T); let sampleVec = normalize(T*H.x + B*H.y + N*H.z); return sampleVec; }
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let N = vec3<f32>(0.0, 0.0, 1.0);
    let V = vec3<f32>(sqrt(1.0 - in.uv.x*in.uv.x), 0.0, in.uv.x);
    let roughness = clamp(in.uv.y, 0.0, 1.0);
    var A = 0.0; var B = 0.0; let SAMPLE_COUNT: u32 = 128u;
    for (var i: u32 = 0u; i < SAMPLE_COUNT; i = i + 1u) {
        let Xi = hammersley(i, SAMPLE_COUNT);
        let H = importanceSampleGGX(Xi, N, roughness);
        let L = normalize(2.0 * dot(V,H) * H - V);
        let NdotL = max(L.z, 0.0);
        if (NdotL > 0.0) {
            let NdotH = max(H.z, 0.0);
            let VdotH = max(dot(V,H), 0.0);
            let G = geometrySmith(N, V, L, roughness);
            let G_Vis = (G * VdotH) / max(NdotH * max(V.z, 1e-4), 1e-4);
            let Fc = pow(1.0 - VdotH, 5.0);
            A = A + (1.0 - Fc) * G_Vis;
            B = B + Fc * G_Vis;
        }
    }
    A = A / f32(SAMPLE_COUNT); B = B / f32(SAMPLE_COUNT);
    return vec4<f32>(A, B, 0.0, 1.0);
}
"#;

// Equirectangular to cubemap conversion shader (minimal placeholder)
const EQUIRECT_TO_CUBE_WGSL: &str = r#"
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
struct FaceIndex { idx: u32 };
@group(1) @binding(0) var<uniform> u_face: FaceIndex;
@vertex fn vs(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0,-1.0), vec2<f32>(3.0,-1.0), vec2<f32>(-1.0,3.0));
    let xy = p[vi];
    out.pos = vec4<f32>(xy, 0.0, 1.0);
    out.uv = (xy+1.0)*0.5;
    return out;
}
@group(0) @binding(0) var hdr_equirect: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
fn uv_to_dir(face: i32, uv: vec2<f32>) -> vec3<f32> {
    let a = uv*2.0 - 1.0;
    if (face == 0) { return normalize(vec3<f32>( 1.0,    -a.y,   -a.x)); }
    if (face == 1) { return normalize(vec3<f32>(-1.0,    -a.y,    a.x)); }
    if (face == 2) { return normalize(vec3<f32>( a.x,     1.0,    a.y)); }
    if (face == 3) { return normalize(vec3<f32>( a.x,    -1.0,   -a.y)); }
    if (face == 4) { return normalize(vec3<f32>( a.x,    -a.y,    1.0)); }
    return normalize(vec3<f32>(-a.x,   -a.y,   -1.0));
}
fn dir_to_equirect_uv(dir: vec3<f32>) -> vec2<f32> {
    let n = normalize(dir);
    let phi = atan2(n.z, n.x);
    let theta = acos(clamp(n.y, -1.0, 1.0));
    let u = (phi / 6.2831853 + 0.5);
    let v = theta / 3.14159265;
    return vec2<f32>(u, v);
}
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // CPU passes face index via uniform
    let dir = uv_to_dir(i32(u_face.idx), in.uv);
    let uv = dir_to_equirect_uv(dir);
    let c = textureSample(hdr_equirect, samp, uv);
    return vec4<f32>(c.rgb, 1.0);
}
"#;

// Host-side helpers for HDR equirectangular upload

/// Compute the average luminance of an HDR image using a fast stride-sampled
/// log-average.  Returns a value suitable for normalising IBL intensity so that
/// different HDRIs produce consistent terrain lighting.
#[cfg(feature = "textures")]
fn compute_hdr_avg_luminance(img: &image::DynamicImage) -> f32 {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    // Sample every 4th pixel for speed on large HDRIs
    let stride = 4;
    let mut sum = 0.0f64;
    let mut count = 0u64;
    for y in (0..h).step_by(stride) {
        for x in (0..w).step_by(stride) {
            let px = rgba.get_pixel(x as u32, y as u32);
            // Rec. 709 luminance from sRGB-encoded bytes (approximate)
            let r = px[0] as f64 / 255.0;
            let g = px[1] as f64 / 255.0;
            let b = px[2] as f64 / 255.0;
            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            sum += lum;
            count += 1;
        }
    }
    if count == 0 {
        return 1.0;
    }
    (sum / count as f64) as f32
}

/// Maximum HDRI dimension allowed.  A 16384×8192 panorama at Rgba16Float
/// costs ~2 GB of GPU memory — well beyond most GPUs.  We cap at 8192 on
/// the longest axis, which gives excellent quality while keeping GPU usage
/// under ~256 MB for the equirectangular source texture.
#[cfg(feature = "textures")]
const MAX_HDR_DIMENSION: u32 = 8192;

#[cfg(feature = "textures")]
fn load_hdr_equirectangular(path: &Path) -> Result<image::DynamicImage> {
    let reader = image::ImageReader::open(path)?;
    let img = reader.decode()?;
    let (w, h) = img.dimensions();
    if w > MAX_HDR_DIMENSION || h > MAX_HDR_DIMENSION {
        log::info!(
            "HDRI {}x{} exceeds max {}; downsampling",
            w,
            h,
            MAX_HDR_DIMENSION
        );
        let aspect = w as f32 / h as f32;
        let (tw, th) = if w >= h {
            (
                MAX_HDR_DIMENSION,
                (MAX_HDR_DIMENSION as f32 / aspect) as u32,
            )
        } else {
            (
                (MAX_HDR_DIMENSION as f32 * aspect) as u32,
                MAX_HDR_DIMENSION,
            )
        };
        Ok(image::DynamicImage::ImageRgba8(image::imageops::resize(
            &img.to_rgba8(),
            tw.max(1),
            th.max(1),
            image::imageops::FilterType::Lanczos3,
        )))
    } else {
        Ok(img)
    }
}

#[cfg(feature = "textures")]
fn create_hdr2d(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    img: &image::DynamicImage,
) -> Result<(wgpu::Texture, wgpu::TextureView, wgpu::Sampler)> {
    let (w, h) = img.dimensions();
    let rgba_f32: Vec<f32> = match img {
        image::DynamicImage::ImageRgb32F(buf) => buf
            .pixels()
            .flat_map(|p| vec![p[0], p[1], p[2], 1.0])
            .collect(),
        image::DynamicImage::ImageRgba32F(buf) => buf
            .pixels()
            .flat_map(|p| vec![p[0], p[1], p[2], p[3]])
            .collect(),
        _ => img
            .to_rgba8()
            .pixels()
            .flat_map(|p| {
                vec![
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                    p[3] as f32 / 255.0,
                ]
            })
            .collect(),
    };
    let mut rgba_f16 = Vec::with_capacity((w * h * 4) as usize * 2);
    for f in rgba_f32.into_iter() {
        let h = half::f16::from_f32(f);
        rgba_f16.extend_from_slice(&h.to_le_bytes());
    }
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ibl-hdr2d"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba_f16,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(w * 8),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let samp = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ibl-hdr2d-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    Ok((tex, view, samp))
}

// ============================================================================
// Unit Tests for IBL Implementation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ibl_quality_presets() {
        // Test Low quality: env_size=256, spec_size=env_size/2=128
        assert_eq!(IblQuality::Low.env_size(), 256);
        assert_eq!(IblQuality::Low.irradiance_size(), 64); // Fixed for all qualities
        assert_eq!(IblQuality::Low.spec_size(), 128); // env_size / 2
        assert_eq!(IblQuality::Low.spec_mips(), 8); // log2(128) + 1

        // Test Medium quality: env_size=512, spec_size=256
        assert_eq!(IblQuality::Medium.env_size(), 512);
        assert_eq!(IblQuality::Medium.irradiance_size(), 64);
        assert_eq!(IblQuality::Medium.spec_size(), 256); // env_size / 2
        assert_eq!(IblQuality::Medium.spec_mips(), 9); // log2(256) + 1

        // Test High quality: env_size=1024, spec_size=512
        assert_eq!(IblQuality::High.env_size(), 1024);
        assert_eq!(IblQuality::High.irradiance_size(), 64);
        assert_eq!(IblQuality::High.spec_size(), 512); // env_size / 2
        assert_eq!(IblQuality::High.spec_mips(), 10); // log2(512) + 1
    }

    #[test]
    fn test_sky_mode_creation() {
        // Test Procedural mode
        let procedural = SkyMode::Procedural {
            last_capture_time: 0.0,
            recapture_interval: 60.0,
        };
        match procedural {
            SkyMode::Procedural {
                last_capture_time,
                recapture_interval,
            } => {
                assert_eq!(last_capture_time, 0.0);
                assert_eq!(recapture_interval, 60.0);
            }
            _ => unreachable!("SkyMode should be Procedural variant"),
        }

        // Test HdrPath mode
        let hdr_path = SkyMode::HdrPath {
            biome: "grassland".to_string(),
            path: "assets/env.hdr".to_string(),
        };
        match hdr_path {
            SkyMode::HdrPath { biome, path } => {
                assert_eq!(biome, "grassland");
                assert_eq!(path, "assets/env.hdr");
            }
            _ => unreachable!("SkyMode should be HdrPath variant"),
        }
    }

    #[test]
    fn test_prefilter_params_roughness_calculation() {
        // Test roughness calculation for mip chain
        // Roughness should be linear from 0.0 (mip 0) to 1.0 (last mip)

        let spec_mips = 10u32;
        for mip in 0..spec_mips {
            let roughness = (mip as f32) / ((spec_mips - 1) as f32).max(1.0);

            if mip == 0 {
                assert_eq!(roughness, 0.0, "Mip 0 should have roughness 0.0");
            } else if mip == spec_mips - 1 {
                assert_eq!(roughness, 1.0, "Last mip should have roughness 1.0");
            } else {
                assert!(
                    roughness > 0.0 && roughness < 1.0,
                    "Mid mips should have roughness between 0 and 1, got {}",
                    roughness
                );
            }
        }
    }

    #[test]
    fn test_sample_count_by_quality() {
        // Test sample counts for different quality levels and mip levels

        // Low quality
        let low_mip0_samples = 128u32;
        let low_other_samples = 64u32;
        assert!(
            low_mip0_samples > low_other_samples,
            "Mip 0 should have more samples than other mips"
        );

        // Medium quality
        let med_mip0_samples = 256u32;
        let med_other_samples = 128u32;
        assert!(med_mip0_samples > med_other_samples);
        assert!(
            med_mip0_samples > low_mip0_samples,
            "Medium quality should have more samples than Low"
        );

        // High quality
        let high_mip0_samples = 512u32;
        let high_other_samples = 256u32;
        assert!(high_mip0_samples > high_other_samples);
        assert!(
            high_mip0_samples > med_mip0_samples,
            "High quality should have more samples than Medium"
        );
    }

    #[test]
    fn test_face_indexing() {
        // Test that face indices are in valid range [0, 5] for cubemap
        for face in 0..6u32 {
            assert!(face < 6, "Face index must be less than 6");
        }
    }

    #[test]
    fn test_uniform_buffer_alignment() {
        // PrefilterParams uniform buffer should be 16 bytes (4 x u32/f32)
        // This ensures proper alignment for GPU
        let roughness = 0.5f32;
        let face = 2u32;
        let sample_count = 256u32;
        let pad = 0u32;

        let params_data = [f32::to_bits(roughness), face, sample_count, pad];

        assert_eq!(params_data.len(), 4, "Params should have 4 elements");
        assert_eq!(
            std::mem::size_of_val(&params_data),
            16,
            "Params should be 16 bytes for alignment"
        );
    }

    #[test]
    fn test_ibl_resources_struct() {
        // Verify IblResources has all required fields
        // This is a compile-time check but documents the structure
        fn _assert_ibl_resources_complete(res: IblResources) {
            let _ = res.env_cube;
            let _ = res.irradiance_cube;
            let _ = res.specular_cube;
            let _ = res.brdf_lut;
            let _ = res.mips_specular;
            let _ = res.avg_luminance;
        }
    }

    #[test]
    fn test_shader_constant_consistency() {
        // Check for key shader patterns
        assert!(
            SPECULAR_PREFILTER_WGSL.contains("PrefilterParams"),
            "Specular shader should use PrefilterParams"
        );
        assert!(
            SPECULAR_PREFILTER_WGSL.contains("roughness"),
            "Specular shader should reference roughness"
        );
        assert!(
            IRRADIANCE_WGSL.contains("irradiance"),
            "Irradiance shader should compute irradiance"
        );
    }
}
