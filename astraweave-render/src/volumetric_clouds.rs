//! Volumetric Clouds — Perlin-Worley Raymarching Pipeline
//!
//! Implements physically-based volumetric clouds following Schneider & Vos
//! (SIGGRAPH 2015) "The Real-Time Volumetric Cloudscapes of Horizon Zero Dawn."
//!
//! Architecture (2-pass compute pipeline):
//!
//! 1. **Raymarch** (half-resolution): March through cloud shell, sample Perlin-Worley
//!    noise for density, compute Beer-Powder lighting with dual-lobe HG phase.
//!    Temporal blend with history for stability.
//! 2. **Composite** (full-resolution): Bilinear upscale from half-res, depth-aware
//!    compositing over the scene (clouds are behind geometry).
//!
//! Cloud noise is fully procedural (no precomputed noise textures):
//! - Base shape: 3-octave FBM of Perlin-Worley noise
//! - Detail erosion: 2-octave Worley FBM
//! - Coverage: analytical weather function modulated by `cloud_coverage`

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::bind_group_cache::{CachedBindGroup, Generation};

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// Raymarching pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CloudParams {
    pub inv_view_proj: [[f32; 4]; 4],
    pub view_pos: [f32; 3],
    pub near_plane: f32,
    pub sun_dir: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub cloud_altitude: f32,
    pub cloud_thickness: f32,
    pub cloud_coverage: f32,
    pub cloud_density: f32,
    pub cloud_speed: f32,
    pub wind_dir: [f32; 3],
    pub time: f32,
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub max_steps: u32,
    pub light_steps: u32,
    pub anisotropy_fwd: f32,
    pub anisotropy_bck: f32,
    pub extinction_coeff: f32,
    pub scatter_coeff: f32,
    pub ambient_intensity: f32,
    pub temporal_blend: f32,
    pub frame_index: u32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

/// Composite pass uniform parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CloudCompositeParams {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub cloud_resolution: [f32; 2],
    pub inv_cloud_resolution: [f32; 2],
    pub near_plane: f32,
    pub far_plane: f32,
    pub cloud_altitude: f32,
    pub cloud_thickness: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for volumetric clouds.
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Enable/disable cloud rendering.
    pub enabled: bool,
    /// Bottom altitude of the cloud layer in world units.
    pub cloud_altitude: f32,
    /// Vertical thickness of the cloud layer.
    pub cloud_thickness: f32,
    /// Cloud coverage fraction (0.0 = clear sky, 1.0 = fully overcast).
    pub cloud_coverage: f32,
    /// Overall density multiplier.
    pub cloud_density: f32,
    /// Wind animation speed.
    pub cloud_speed: f32,
    /// Wind direction (normalized).
    pub wind_dir: Vec3,
    /// Maximum primary ray steps (32–64 recommended).
    pub max_steps: u32,
    /// Light march steps toward sun (4–8 recommended).
    pub light_steps: u32,
    /// Forward scattering anisotropy for silver lining (0.6–0.9).
    pub anisotropy_forward: f32,
    /// Backward scattering anisotropy for dark edges (-0.3 to -0.6).
    pub anisotropy_backward: f32,
    /// Extinction coefficient (how quickly light is absorbed).
    pub extinction_coeff: f32,
    /// Scattering coefficient (how much light is scattered in-view).
    pub scatter_coeff: f32,
    /// Ambient light intensity inside clouds.
    pub ambient_intensity: f32,
    /// Temporal blend factor (0.9–0.97). Higher = smoother but more ghosting.
    pub temporal_blend: f32,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cloud_altitude: 1000.0,
            cloud_thickness: 600.0,
            cloud_coverage: 0.5,
            cloud_density: 0.8,
            cloud_speed: 0.02,
            wind_dir: Vec3::new(1.0, 0.0, 0.3).normalize(),
            max_steps: 48,
            light_steps: 4,
            anisotropy_forward: 0.8,
            anisotropy_backward: -0.5,
            extinction_coeff: 0.04,
            scatter_coeff: 0.04,
            ambient_intensity: 0.15,
            temporal_blend: 0.95,
        }
    }
}

/// Cloud quality presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudQuality {
    /// 32 primary steps, 3 light steps
    Low,
    /// 48 primary steps, 4 light steps (default)
    Medium,
    /// 64 primary steps, 6 light steps
    High,
}

impl CloudQuality {
    /// Get the (primary_steps, light_steps) for this quality level.
    pub fn step_counts(self) -> (u32, u32) {
        match self {
            CloudQuality::Low => (32, 3),
            CloudQuality::Medium => (48, 4),
            CloudQuality::High => (64, 6),
        }
    }
}

// ---------------------------------------------------------------------------
// Volumetric Clouds Pass
// ---------------------------------------------------------------------------

/// Manages the volumetric cloud rendering pipeline (raymarch + composite).
pub struct VolumetricCloudsPass {
    config: CloudConfig,
    // Pipelines
    raymarch_pipeline: wgpu::ComputePipeline,
    composite_pipeline: wgpu::ComputePipeline,
    // Bind group layouts
    raymarch_bgl: wgpu::BindGroupLayout,
    composite_bgl: wgpu::BindGroupLayout,
    // Uniform buffers
    cloud_params_buf: wgpu::Buffer,
    composite_params_buf: wgpu::Buffer,
    // Half-res cloud textures
    cloud_texture: wgpu::Texture,
    cloud_view: wgpu::TextureView,
    cloud_history: wgpu::Texture,
    cloud_history_view: wgpu::TextureView,
    // Full-res composited output
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    composited_texture: wgpu::Texture,
    composited_view: wgpu::TextureView,
    // Sampler
    linear_sampler: wgpu::Sampler,
    // State
    frame_index: u32,
    time: f32,
    half_width: u32,
    half_height: u32,
    width: u32,
    height: u32,
    // Cached bind groups
    cached_raymarch_bg: CachedBindGroup,
    cached_composite_bg: CachedBindGroup,
}

impl VolumetricCloudsPass {
    /// Create a new volumetric clouds pass.
    pub fn new(device: &wgpu::Device, width: u32, height: u32, config: CloudConfig) -> Self {
        let half_width = width.div_ceil(2);
        let half_height = height.div_ceil(2);
        let fmt = wgpu::TextureFormat::Rgba16Float;

        // --- Half-res textures ---
        let make_half = |label: &str| {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: half_width,
                    height: half_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            (tex, view)
        };
        let (cloud_texture, cloud_view) = make_half("cloud_raymarch");
        let (cloud_history, cloud_history_view) = make_half("cloud_history");

        // --- Full-res composited output ---
        let composited_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud_composited"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let composited_view =
            composited_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // --- Uniform buffers ---
        let cloud_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cloud_params"),
            contents: &[0u8; std::mem::size_of::<CloudParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let composite_params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cloud_composite_params"),
            contents: &[0u8; std::mem::size_of::<CloudCompositeParams>()],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // --- Bind group layouts ---
        let raymarch_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud_raymarch_bgl"),
            entries: &[
                bgl_uniform(0),            // CloudParams
                bgl_texture_2d(1),         // history
                bgl_sampler(2),            // linear sampler
                bgl_storage_2d_rw(3, fmt), // output
            ],
        });

        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cloud_composite_bgl"),
            entries: &[
                bgl_uniform(0),            // CloudCompositeParams
                bgl_texture_2d(1),         // scene color
                bgl_texture_2d(2),         // cloud result
                bgl_texture_2d(3),         // depth
                bgl_sampler(4),            // linear sampler
                bgl_storage_2d_rw(5, fmt), // output
            ],
        });

        // --- Pipelines ---
        let raymarch_pipeline = create_compute_pipeline(
            device,
            &raymarch_bgl,
            "cloud_raymarch",
            include_str!("../shaders/volumetrics/cloud_raymarching.wgsl"),
            "cloud_raymarch_main",
        );
        let composite_pipeline = create_compute_pipeline(
            device,
            &composite_bgl,
            "cloud_composite",
            include_str!("../shaders/volumetrics/cloud_composite.wgsl"),
            "cloud_composite_main",
        );

        // --- Sampler ---
        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cloud_linear_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        Self {
            config,
            raymarch_pipeline,
            composite_pipeline,
            raymarch_bgl,
            composite_bgl,
            cloud_params_buf,
            composite_params_buf,
            cloud_texture,
            cloud_view,
            cloud_history,
            cloud_history_view,
            composited_texture,
            composited_view,
            linear_sampler,
            frame_index: 0,
            time: 0.0,
            half_width,
            half_height,
            width,
            height,
            cached_raymarch_bg: CachedBindGroup::new(),
            cached_composite_bg: CachedBindGroup::new(),
        }
    }

    /// Get current cloud configuration.
    pub fn config(&self) -> &CloudConfig {
        &self.config
    }

    /// Update cloud configuration.
    pub fn set_config(&mut self, config: CloudConfig) {
        self.config = config;
    }

    /// Get the final composited output (full-res, clouds + scene).
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.composited_view
    }

    /// Get the raw cloud raymarching result (half-res).
    pub fn cloud_view(&self) -> &wgpu::TextureView {
        &self.cloud_view
    }

    /// Get the screen dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Update uniform buffers for the current frame.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_frame(
        &mut self,
        queue: &wgpu::Queue,
        inv_view_proj: Mat4,
        view_pos: Vec3,
        sun_dir: Vec3,
        sun_color: Vec3,
        sun_intensity: f32,
        _ambient_intensity: f32,
        near: f32,
        far: f32,
        dt: f32,
    ) {
        self.time += dt;

        let params = CloudParams {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            view_pos: view_pos.to_array(),
            near_plane: near,
            sun_dir: sun_dir.to_array(),
            sun_intensity,
            sun_color: sun_color.to_array(),
            cloud_altitude: self.config.cloud_altitude,
            cloud_thickness: self.config.cloud_thickness,
            cloud_coverage: self.config.cloud_coverage,
            cloud_density: self.config.cloud_density,
            cloud_speed: self.config.cloud_speed,
            wind_dir: self.config.wind_dir.to_array(),
            time: self.time,
            resolution: [self.half_width as f32, self.half_height as f32],
            inv_resolution: [1.0 / self.half_width as f32, 1.0 / self.half_height as f32],
            max_steps: self.config.max_steps,
            light_steps: self.config.light_steps,
            anisotropy_fwd: self.config.anisotropy_forward,
            anisotropy_bck: self.config.anisotropy_backward,
            extinction_coeff: self.config.extinction_coeff,
            scatter_coeff: self.config.scatter_coeff,
            ambient_intensity: self.config.ambient_intensity,
            temporal_blend: self.config.temporal_blend,
            frame_index: self.frame_index,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        queue.write_buffer(&self.cloud_params_buf, 0, bytemuck::bytes_of(&params));

        let composite_params = CloudCompositeParams {
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            cloud_resolution: [self.half_width as f32, self.half_height as f32],
            inv_cloud_resolution: [1.0 / self.half_width as f32, 1.0 / self.half_height as f32],
            near_plane: near,
            far_plane: far,
            cloud_altitude: self.config.cloud_altitude,
            cloud_thickness: self.config.cloud_thickness,
        };
        queue.write_buffer(
            &self.composite_params_buf,
            0,
            bytemuck::bytes_of(&composite_params),
        );

        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Execute the full volumetric cloud pipeline.
    ///
    /// `resource_gen` is the renderer's current generation counter; bind groups
    /// are rebuilt only when it changes (e.g., after a resize).
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        resource_gen: Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        // --- Pass 1: Raymarch (half-resolution) ---
        if !self.cached_raymarch_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud_raymarch_bg"),
                layout: &self.raymarch_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.cloud_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.cloud_history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.cloud_view),
                    },
                ],
            });
            self.cached_raymarch_bg = CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cloud_raymarch"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.raymarch_pipeline);
            let bg = self
                .cached_raymarch_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(self.half_width.div_ceil(8), self.half_height.div_ceil(8), 1);
        }

        // --- Pass 2: Composite (full-resolution) ---
        if !self.cached_composite_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud_composite_bg"),
                layout: &self.composite_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.composite_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(scene_color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.cloud_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&self.composited_view),
                    },
                ],
            });
            self.cached_composite_bg = CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cloud_composite"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            let bg = self
                .cached_composite_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }
    }

    /// Copy current cloud output to history for next frame's temporal blend.
    pub fn copy_cloud_to_history(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.cloud_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.cloud_history,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.half_width,
                height: self.half_height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Resize textures when the screen size changes.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        let config = self.config.clone();
        *self = Self::new(device, width, height, config);
    }
}

// ---------------------------------------------------------------------------
// Pipeline / BGL helpers (local to this module)
// ---------------------------------------------------------------------------

fn create_compute_pipeline(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    label: &str,
    source: &str,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label}_pl")),
        bind_group_layouts: &[bgl],
        push_constant_ranges: &[],
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(&format!("{label}_pipeline")),
        layout: Some(&pl),
        module: &shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
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

fn bgl_texture_2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

fn bgl_storage_2d_rw(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_params_size() {
        // mat4(64) + vec3+f32(16) + vec3+f32(16) + vec3+f32(16)
        // + 4×f32(16) + vec3+f32(16) + vec2+vec2(16) + 4×scalars(16)
        // + 4×scalars(16) + u32+3×pad(16) = 208
        assert_eq!(std::mem::size_of::<CloudParams>(), 208);
    }

    #[test]
    fn cloud_composite_params_size() {
        // 2×vec2(16) + 2×vec2(16) + 4×f32(16) = 48
        assert_eq!(std::mem::size_of::<CloudCompositeParams>(), 48);
    }

    #[test]
    fn default_config() {
        let c = CloudConfig::default();
        assert!(c.enabled);
        assert_eq!(c.cloud_altitude, 1000.0);
        assert!(c.cloud_thickness > 0.0);
        assert!((c.cloud_coverage - 0.5).abs() < 1e-6);
        assert!(c.max_steps >= 32);
        assert!(c.light_steps >= 3);
        assert!(
            c.anisotropy_forward > 0.0,
            "Forward scattering should be positive"
        );
        assert!(
            c.anisotropy_backward < 0.0,
            "Backward scattering should be negative"
        );
        assert!(c.temporal_blend > 0.8 && c.temporal_blend < 1.0);
    }

    #[test]
    fn quality_presets() {
        let (low_p, low_l) = CloudQuality::Low.step_counts();
        let (med_p, med_l) = CloudQuality::Medium.step_counts();
        let (high_p, high_l) = CloudQuality::High.step_counts();
        assert!(low_p < med_p);
        assert!(med_p < high_p);
        assert!(low_l <= med_l);
        assert!(med_l <= high_l);
    }

    #[test]
    fn parse_cloud_raymarching_wgsl() {
        let source = include_str!("../shaders/volumetrics/cloud_raymarching.wgsl");
        let module = naga::front::wgsl::parse_str(source);
        assert!(
            module.is_ok(),
            "cloud_raymarching.wgsl parse failed: {:?}",
            module.err()
        );
    }

    #[test]
    fn parse_cloud_composite_wgsl() {
        let source = include_str!("../shaders/volumetrics/cloud_composite.wgsl");
        let module = naga::front::wgsl::parse_str(source);
        assert!(
            module.is_ok(),
            "cloud_composite.wgsl parse failed: {:?}",
            module.err()
        );
    }

    #[test]
    fn cloud_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));
        let adapter = match adapter {
            Ok(a) => a,
            Err(_) => {
                eprintln!("SKIP: no GPU adapter available");
                return;
            }
        };
        let (device, _queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("cloud_test"),
                ..Default::default()
            }))
            .unwrap();

        let config = CloudConfig::default();
        let pass = VolumetricCloudsPass::new(&device, 1920, 1080, config);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert_eq!(pass.half_width, 960);
        assert_eq!(pass.half_height, 540);
    }
}
