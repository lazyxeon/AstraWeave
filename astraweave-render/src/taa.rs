//! Temporal Anti-Aliasing (TAA) with anti-ghosting.
//!
//! Provides production TAA: Halton(2,3) jitter sequence, history reprojection
//! via motion vectors, YCoCg neighborhood clamping, Catmull-Rom history sampling,
//! velocity-dependent blending, and optional RCAS sharpening.

/// GPU-side uniform for the TAA resolve pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TaaUniforms {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    /// x: history blend factor, y: clamp margin, z: sharpen strength, w: frame index
    pub config: [f32; 4],
}

/// TAA configuration.
#[derive(Debug, Clone)]
pub struct TaaConfig {
    pub enabled: bool,
    /// History blend factor (0.9 = responsive, 0.98 = very stable).
    pub blend_factor: f32,
    /// AABB clamp margin (larger = less ghosting but more aliasing).
    pub clamp_margin: f32,
    /// Jitter scale multiplier.
    pub jitter_scale: f32,
    /// RCAS sharpening strength (0 = off, 0.5 = moderate, 1.0 = strong).
    pub sharpen_strength: f32,
    /// Number of jitter samples in the Halton sequence (typically 8 or 16).
    pub jitter_samples: u32,
}

impl Default for TaaConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            blend_factor: 0.95,
            clamp_margin: 0.02,
            jitter_scale: 1.0,
            sharpen_strength: 0.5,
            jitter_samples: 8,
        }
    }
}

/// Halton sequence for sub-pixel jitter.
/// Low-discrepancy sequence that provides good coverage over N frames.
pub fn halton(index: u32, base: u32) -> f32 {
    let mut result = 0.0f32;
    let mut f = 1.0f32;
    let mut i = index;
    let b = base as f32;

    while i > 0 {
        f /= b;
        result += f * (i % base) as f32;
        i /= base;
    }

    result
}

/// Get the jitter offset for a given frame index.
/// Returns (x, y) in pixel units, centered around 0.
pub fn get_jitter(frame_index: u32, jitter_samples: u32, jitter_scale: f32) -> (f32, f32) {
    let idx = (frame_index % jitter_samples) + 1; // Halton(0) = 0, start from 1
    let jx = (halton(idx, 2) - 0.5) * jitter_scale;
    let jy = (halton(idx, 3) - 0.5) * jitter_scale;
    (jx, jy)
}

/// Apply TAA jitter to a projection matrix.
/// Offsets the projection by sub-pixel amounts to enable temporal supersampling.
pub fn apply_jitter_to_projection(
    proj: glam::Mat4,
    jitter_x: f32,
    jitter_y: f32,
    width: f32,
    height: f32,
) -> glam::Mat4 {
    let mut jittered = proj;
    // Convert pixel offset to NDC offset
    let ndc_x = 2.0 * jitter_x / width;
    let ndc_y = 2.0 * jitter_y / height;
    // Add to the projection matrix's translation (column 2, rows 0 and 1)
    let cols = jittered.to_cols_array_2d();
    let mut modified = cols;
    modified[2][0] += ndc_x;
    modified[2][1] += ndc_y;
    jittered = glam::Mat4::from_cols_array_2d(&modified);
    jittered
}

/// Manages TAA GPU resources: history buffers, compute pipelines, jitter state.
pub struct TaaPass {
    config: TaaConfig,
    /// TAA resolve compute pipeline.
    resolve_pipeline: wgpu::ComputePipeline,
    /// RCAS sharpen compute pipeline.
    sharpen_pipeline: wgpu::ComputePipeline,
    /// Uniform buffer.
    params_buf: wgpu::Buffer,
    /// History texture (previous resolved frame).
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    /// Resolved output texture.
    resolved_texture: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    /// Bind group layout.
    bgl: wgpu::BindGroupLayout,
    /// Frame counter for jitter sequence.
    frame_index: u32,
    width: u32,
    height: u32,
}

impl TaaPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = TaaConfig::default();
        let fmt = wgpu::TextureFormat::Rgba16Float;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let make_tex = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let history_texture = make_tex("taa_history");
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_texture = make_tex("taa_resolved");
        let resolved_view = resolved_texture.create_view(&wgpu::TextureViewDescriptor::default());

        use wgpu::util::DeviceExt;
        let uniforms = TaaUniforms {
            resolution: [width as f32, height as f32],
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            config: [config.blend_factor, config.clamp_margin, config.sharpen_strength, 0.0],
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("taa_params"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa_bgl"),
            entries: &[
                tex_entry(0),      // current / sharpen input
                tex_entry(1),      // history
                tex_entry(2),      // velocity
                tex_entry(3),      // depth
                sampler_entry(4),  // sampler
                uniform_entry(5),  // params
                storage_entry(6, fmt), // output
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("taa_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/taa.wgsl").into()),
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("taa_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let resolve_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("taa_resolve"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("taa_resolve"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sharpen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("taa_rcas"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("rcas_sharpen"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            resolve_pipeline,
            sharpen_pipeline,
            params_buf,
            history_texture,
            history_view,
            resolved_texture,
            resolved_view,
            bgl,
            frame_index: 0,
            width,
            height,
        }
    }

    /// Get the current jitter offset in pixels.
    pub fn current_jitter(&self) -> (f32, f32) {
        get_jitter(self.frame_index, self.config.jitter_samples, self.config.jitter_scale)
    }

    /// Get the resolved output view.
    pub fn resolved_view(&self) -> &wgpu::TextureView {
        &self.resolved_view
    }

    pub fn config(&self) -> &TaaConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: TaaConfig) {
        self.config = config;
    }

    /// Advance to next frame. Call at the end of each frame.
    pub fn next_frame(&mut self) {
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    pub fn frame_index(&self) -> u32 {
        self.frame_index
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update uniforms for this frame.
    pub fn update_params(&self, queue: &wgpu::Queue) {
        let uniforms = TaaUniforms {
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            config: [
                self.config.blend_factor,
                self.config.clamp_margin,
                self.config.sharpen_strength,
                self.frame_index as f32,
            ],
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

fn tex_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn storage_entry(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
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
    fn taa_uniforms_size() {
        assert_eq!(std::mem::size_of::<TaaUniforms>(), 32);
    }

    #[test]
    fn taa_config_default() {
        let c = TaaConfig::default();
        assert!(c.enabled);
        assert!((c.blend_factor - 0.95).abs() < 1e-5);
        assert_eq!(c.jitter_samples, 8);
    }

    #[test]
    fn halton_base2() {
        assert!((halton(1, 2) - 0.5).abs() < 1e-6);
        assert!((halton(2, 2) - 0.25).abs() < 1e-6);
        assert!((halton(3, 2) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn halton_base3() {
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-6);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn jitter_centered() {
        // Jitter values should be in [-0.5, 0.5] range (at scale 1.0)
        for i in 0..16 {
            let (jx, jy) = get_jitter(i, 8, 1.0);
            assert!(jx >= -0.5 && jx <= 0.5, "jx={} out of range", jx);
            assert!(jy >= -0.5 && jy <= 0.5, "jy={} out of range", jy);
        }
    }

    #[test]
    fn apply_jitter_modifies_projection() {
        let proj = glam::Mat4::perspective_rh(std::f32::consts::FRAC_PI_3, 16.0 / 9.0, 0.1, 200.0);
        let jittered = apply_jitter_to_projection(proj, 0.5, 0.3, 1920.0, 1080.0);
        // The matrices should differ in column 2
        assert_ne!(proj, jittered);
    }

    #[test]
    fn taa_pass_creation() {
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

        let pass = TaaPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert!(pass.config().enabled);
    }

    #[test]
    fn taa_next_frame_advances() {
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

        let mut pass = TaaPass::new(&device, 800, 600);
        assert_eq!(pass.frame_index(), 0);
        pass.next_frame();
        assert_eq!(pass.frame_index(), 1);
    }
}
