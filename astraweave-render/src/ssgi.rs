//! Screen-Space Global Illumination (SSGI) with temporal reprojection.
//!
//! Provides one-bounce indirect diffuse lighting by tracing rays in screen space
//! against the depth buffer. Includes spatial bilateral filtering and temporal
//! accumulation via motion vectors for stable, noise-free results.

/// GPU-side uniform parameters for the SSGI compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsgiParams {
    pub inv_proj: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub max_ray_distance: f32,
    pub ray_step_size: f32,
    pub num_rays: u32,
    pub max_steps: u32,
    pub thickness: f32,
    pub intensity: f32,
    pub frame_index: u32,
    pub _pad: u32,
}

impl Default for SsgiParams {
    fn default() -> Self {
        Self {
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            resolution: [1920.0, 1080.0],
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            max_ray_distance: 50.0,
            ray_step_size: 0.5,
            num_rays: 4,
            max_steps: 32,
            thickness: 0.5,
            intensity: 1.0,
            frame_index: 0,
            _pad: 0,
        }
    }
}

/// GPU-side uniform for SSGI temporal denoise.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsgiDenoiseParams {
    pub inv_resolution: [f32; 2],
    pub spatial_sigma: f32,
    pub temporal_blend: f32,
    pub depth_threshold: f32,
    pub normal_threshold: f32,
    pub _pad: [f32; 2],
}

impl Default for SsgiDenoiseParams {
    fn default() -> Self {
        Self {
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            spatial_sigma: 1.5,
            temporal_blend: 0.9,
            depth_threshold: 0.05,
            normal_threshold: 0.8,
            _pad: [0.0; 2],
        }
    }
}

/// Configuration for the SSGI system.
#[derive(Debug, Clone)]
pub struct SsgiConfig {
    pub enabled: bool,
    pub num_rays: u32,
    pub max_steps: u32,
    pub ray_step_size: f32,
    pub thickness: f32,
    pub intensity: f32,
    pub temporal_blend: f32,
}

impl Default for SsgiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_rays: 4,
            max_steps: 32,
            ray_step_size: 0.5,
            thickness: 0.5,
            intensity: 1.0,
            temporal_blend: 0.9,
        }
    }
}

/// Manages SSGI GPU resources.
#[allow(dead_code)]
pub struct SsgiPass {
    config: SsgiConfig,
    /// SSGI compute pipeline.
    gi_pipeline: wgpu::ComputePipeline,
    /// Denoise compute pipeline.
    denoise_pipeline: wgpu::ComputePipeline,
    /// SSGI params buffer.
    params_buf: wgpu::Buffer,
    /// Denoise params buffer.
    denoise_buf: wgpu::Buffer,
    /// Current frame GI output (rgba16float).
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    gi_texture: wgpu::Texture,
    gi_view: wgpu::TextureView,
    /// History GI texture for temporal accumulation.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    /// Denoised output.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    denoised_texture: wgpu::Texture,
    denoised_view: wgpu::TextureView,
    /// Bind group layouts.
    gi_bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    denoise_bgl: wgpu::BindGroupLayout,
    frame_index: u32,
    width: u32,
    height: u32,
}

impl SsgiPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = SsgiConfig::default();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let gi_fmt = wgpu::TextureFormat::Rgba16Float;

        let make_tex = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: gi_fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let gi_texture = make_tex("ssgi_output");
        let gi_view = gi_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let history_texture = make_tex("ssgi_history");
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let denoised_texture = make_tex("ssgi_denoised");
        let denoised_view = denoised_texture.create_view(&wgpu::TextureViewDescriptor::default());

        use wgpu::util::DeviceExt;
        let params = SsgiParams::default();
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssgi_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let denoise_params = SsgiDenoiseParams::default();
        let denoise_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ssgi_denoise_params"),
            contents: bytemuck::bytes_of(&denoise_params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // GI bind group layout
        let gi_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi_gi_bgl"),
            entries: &[
                bgl_texture(0),            // depth
                bgl_texture(1),            // normals
                bgl_texture(2),            // color
                bgl_sampler(3),            // sampler
                bgl_uniform(4),            // params
                bgl_storage_rw(5, gi_fmt), // output
            ],
        });

        // Denoise bind group layout
        let denoise_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ssgi_denoise_bgl"),
            entries: &[
                bgl_texture(0),            // gi current
                bgl_texture(1),            // gi history
                bgl_texture(2),            // depth
                bgl_texture(3),            // velocity
                bgl_sampler(4),            // sampler
                bgl_uniform(5),            // denoise params
                bgl_storage_rw(6, gi_fmt), // output
            ],
        });

        // Pipelines
        let gi_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssgi.wgsl").into()),
        });
        let gi_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi_gi_pl"),
            bind_group_layouts: &[&gi_bgl],
            push_constant_ranges: &[],
        });
        let gi_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssgi_gi_pipeline"),
            layout: Some(&gi_pl),
            module: &gi_shader,
            entry_point: Some("ssgi_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let denoise_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssgi_denoise_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/ssgi_denoise.wgsl").into()),
        });
        let denoise_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ssgi_denoise_pl"),
            bind_group_layouts: &[&denoise_bgl],
            push_constant_ranges: &[],
        });
        let denoise_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ssgi_denoise_pipeline"),
            layout: Some(&denoise_pl),
            module: &denoise_shader,
            entry_point: Some("denoise_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            gi_pipeline,
            denoise_pipeline,
            params_buf,
            denoise_buf,
            gi_texture,
            gi_view,
            history_texture,
            history_view,
            denoised_texture,
            denoised_view,
            gi_bgl,
            denoise_bgl,
            frame_index: 0,
            width,
            height,
        }
    }

    /// Get the final denoised GI texture view.
    #[allow(clippy::misnamed_getters)]
    pub fn gi_view(&self) -> &wgpu::TextureView {
        &self.denoised_view
    }

    pub fn config(&self) -> &SsgiConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: SsgiConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update parameters for this frame.
    pub fn update_params(&mut self, queue: &wgpu::Queue, proj: glam::Mat4) {
        let params = SsgiParams {
            inv_proj: proj.inverse().to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            max_ray_distance: 50.0,
            ray_step_size: self.config.ray_step_size,
            num_rays: self.config.num_rays,
            max_steps: self.config.max_steps,
            thickness: self.config.thickness,
            intensity: self.config.intensity,
            frame_index: self.frame_index,
            _pad: 0,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        let denoise = SsgiDenoiseParams {
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            temporal_blend: self.config.temporal_blend,
            ..SsgiDenoiseParams::default()
        };
        queue.write_buffer(&self.denoise_buf, 0, bytemuck::bytes_of(&denoise));
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Resize all SSGI textures.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

// Helper functions for bind group layout entries
fn bgl_texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn bgl_sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn bgl_uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bgl_storage_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssgi_params_size() {
        // 2 mat4x4 (128) + 4 floats (16) + 4 floats (16) + 4 u32/f32 (16) = 176
        assert_eq!(std::mem::size_of::<SsgiParams>(), 176);
    }

    #[test]
    fn ssgi_denoise_params_size() {
        assert_eq!(std::mem::size_of::<SsgiDenoiseParams>(), 32);
    }

    #[test]
    fn ssgi_config_default() {
        let c = SsgiConfig::default();
        assert!(c.enabled);
        assert_eq!(c.num_rays, 4);
        assert_eq!(c.max_steps, 32);
    }

    #[test]
    fn ssgi_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = SsgiPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
    }
}
