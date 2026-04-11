//! Screen-space God Rays (Crepuscular Rays / Light Shafts).
//!
//! Projects the sun position to screen space and performs radial blur
//! from the sun, sampling the depth buffer to determine occlusion.
//! Produces bright shafts where light passes between occluders.
//!
//! The effect is additive — it should be composited after the main
//! lighting pass (and optionally after volumetric fog).

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// GPU types
// ---------------------------------------------------------------------------

/// GPU-side uniform parameters for the god rays compute pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GodRayParams {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub sun_screen_pos: [f32; 2],
    pub sun_visible: f32,
    pub num_samples: u32,
    pub density: f32,
    pub weight: f32,
    pub decay: f32,
    pub exposure: f32,
    pub sun_color: [f32; 3],
    pub _pad: f32,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for god rays.
#[derive(Debug, Clone)]
pub struct GodRayConfig {
    pub enabled: bool,
    /// Number of radial samples per pixel (32–128).
    pub num_samples: u32,
    /// Ray density multiplier (controls ray length).
    pub density: f32,
    /// Per-sample contribution weight.
    pub weight: f32,
    /// Exponential decay per sample (0.9–0.99).
    pub decay: f32,
    /// Final brightness multiplier.
    pub exposure: f32,
    /// Render at half resolution for performance (bilinear upscale on composite).
    pub half_res: bool,
}

impl Default for GodRayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            num_samples: 48,
            density: 0.96,
            weight: 0.25,
            decay: 0.97,
            exposure: 0.3,
            half_res: false,
        }
    }
}

// ---------------------------------------------------------------------------
// God Ray Pass
// ---------------------------------------------------------------------------

/// Manages GPU resources for the god rays compute pass.
pub struct GodRayPass {
    config: GodRayConfig,
    pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    params_buf: wgpu::Buffer,
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    output_texture: wgpu::Texture,
    output_view: wgpu::TextureView,
    width: u32,
    height: u32,
    /// Cached sampler (static — never changes).
    sampler: wgpu::Sampler,
    /// Cached bind group (rebuilt on generation change).
    cached_bg: crate::bind_group_cache::CachedBindGroup,
}

impl GodRayPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::with_config(device, width, height, GodRayConfig::default())
    }

    pub fn with_config(device: &wgpu::Device, width: u32, height: u32, config: GodRayConfig) -> Self {
        // Compute effective resolution (half-res if enabled)
        let tex_w = if config.half_res { width.div_ceil(2) } else { width };
        let tex_h = if config.half_res { height.div_ceil(2) } else { height };
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let output_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("god_rays_output"),
            size: wgpu::Extent3d {
                width: tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let params = GodRayParams {
            resolution: [tex_w as f32, tex_h as f32],
            inv_resolution: [1.0 / tex_w as f32, 1.0 / tex_h as f32],
            sun_screen_pos: [0.5, 0.5],
            sun_visible: 0.0,
            num_samples: config.num_samples,
            density: config.density,
            weight: config.weight,
            decay: config.decay,
            exposure: config.exposure,
            sun_color: [1.0, 0.95, 0.85],
            _pad: 0.0,
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("god_ray_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("god_ray_bgl"),
            entries: &[
                // 0: params
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 1: depth (Depth32Float)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 2: scene color
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 3: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 4: output
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: fmt,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("god_rays_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/volumetrics/god_rays.wgsl").into(),
            ),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("god_rays_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("god_rays_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("god_rays_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            pipeline,
            bgl,
            params_buf,
            output_texture,
            output_view,
            width: tex_w,
            height: tex_h,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("god_rays_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            cached_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Get the god rays output view (additive light shafts).
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.output_view
    }

    pub fn config(&self) -> &GodRayConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: GodRayConfig) {
        self.config = config;
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Project the sun direction to screen space and update parameters.
    pub fn update_params(
        &self,
        queue: &wgpu::Queue,
        view_proj: Mat4,
        sun_dir: Vec3,
        sun_color: Vec3,
    ) {
        // Project a distant point in the sun direction to screen space
        let sun_world = sun_dir * 10000.0; // far enough to be "at infinity"
        let clip = view_proj * sun_world.extend(1.0);

        let (sun_screen, visible) = if clip.w > 0.0 {
            let ndc = glam::Vec2::new(clip.x / clip.w, clip.y / clip.w);
            let uv = ndc * 0.5 + 0.5;
            // Check if on screen (with some margin)
            let on_screen = uv.x >= -0.2 && uv.x <= 1.2 && uv.y >= -0.2 && uv.y <= 1.2;
            (uv.to_array(), if on_screen { 1.0_f32 } else { 0.0 })
        } else {
            ([0.5, 0.5], 0.0)
        };

        let params = GodRayParams {
            resolution: [self.width as f32, self.height as f32],
            inv_resolution: [1.0 / self.width as f32, 1.0 / self.height as f32],
            sun_screen_pos: sun_screen,
            sun_visible: visible,
            num_samples: self.config.num_samples,
            density: self.config.density,
            weight: self.config.weight,
            decay: self.config.decay,
            exposure: self.config.exposure,
            sun_color: sun_color.to_array(),
            _pad: 0.0,
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
    }

    /// Dispatch the god rays compute pass.
    ///
    /// `resource_gen` is the renderer's generation counter; the bind group is
    /// rebuilt only when it changes (e.g., after a resize).
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        scene_color_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        if !self.cached_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("god_rays_bg"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(scene_color_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.output_view),
                    },
                ],
            });
            self.cached_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        let wg_x = self.width.div_ceil(8);
        let wg_y = self.height.div_ceil(8);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("god_rays"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        let bg = self
            .cached_bg
            .get_or_rebuild(resource_gen, || unreachable!());
        pass.set_bind_group(0, bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    /// Resize output texture (preserves current config including half_res).
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let tex_w = if self.config.half_res { width.div_ceil(2) } else { width };
        let tex_h = if self.config.half_res { height.div_ceil(2) } else { height };
        if self.width == tex_w && self.height == tex_h {
            return;
        }
        let config = self.config.clone();
        *self = Self::with_config(device, width, height, config);
    }
}

/// Project a sun direction to screen-space UV (CPU utility).
pub fn sun_to_screen(view_proj: Mat4, sun_dir: Vec3) -> Option<[f32; 2]> {
    let clip = view_proj * (sun_dir * 10000.0).extend(1.0);
    if clip.w <= 0.0 {
        return None;
    }
    let ndc = glam::Vec2::new(clip.x / clip.w, clip.y / clip.w);
    let uv = ndc * 0.5 + 0.5;
    if uv.x >= -0.2 && uv.x <= 1.2 && uv.y >= -0.2 && uv.y <= 1.2 {
        Some(uv.to_array())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn god_ray_params_size() {
        assert_eq!(std::mem::size_of::<GodRayParams>(), 64);
    }

    #[test]
    fn default_config() {
        let c = GodRayConfig::default();
        assert!(c.enabled);
        assert_eq!(c.num_samples, 48);
        assert!(c.decay > 0.9 && c.decay < 1.0);
        assert!(c.exposure > 0.0);
    }

    #[test]
    fn sun_to_screen_behind_camera() {
        // Sun behind camera (negative Z in view space = behind)
        let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 1000.0)
            * Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        // Sun in +Z direction (behind camera looking at -Z)
        let result = sun_to_screen(vp, Vec3::Z);
        assert!(result.is_none(), "Sun behind camera should be None");
    }

    #[test]
    fn sun_to_screen_in_front() {
        let vp = Mat4::perspective_rh(1.0, 1.0, 0.1, 1000.0)
            * Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        // Sun in -Z direction (in front of camera)
        let result = sun_to_screen(vp, Vec3::NEG_Z);
        assert!(result.is_some(), "Sun in front should be visible");
        let [u, v] = result.unwrap();
        // Should be near center of screen
        assert!((u - 0.5).abs() < 0.1, "u={u}");
        assert!((v - 0.5).abs() < 0.1, "v={v}");
    }

    #[test]
    fn god_ray_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("adapter");
        let (device, _) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("device");

        let pass = GodRayPass::new(&device, 640, 480);
        assert_eq!(pass.dimensions(), (640, 480));
    }

    #[test]
    fn decay_convergence() {
        // Verify that decay^num_samples → near zero (rays fade out)
        let c = GodRayConfig::default();
        let final_weight = c.decay.powi(c.num_samples as i32);
        assert!(
            final_weight < 0.5,
            "Rays should fade significantly: {final_weight}"
        );
    }
}
