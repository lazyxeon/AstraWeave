//! Temporal Upsampling (TAA-U) — Custom Temporal Upscaler
//!
//! Renders at a configurable internal resolution (50-100% of display), then
//! accumulates sub-pixel detail at native display resolution over time using
//! TAA-style temporal reprojection. Each frame's jittered sub-pixel offset
//! samples a unique position within the display texel, gradually building
//! native-resolution detail in the history buffer.
//!
//! Pipeline: reduced-res render → upscale resolve → RCAS sharpen → output
//!
//! Integrates with the existing TAA jitter infrastructure ([`super::taa`]).

/// GPU-side uniform struct for the temporal upscale pass.
/// Must match `UpscaleParams` in `temporal_upscale.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UpscaleParams {
    /// Display (output) resolution in pixels.
    pub output_resolution: [f32; 2],
    /// 1.0 / output_resolution.
    pub output_inv_resolution: [f32; 2],
    /// Internal (render) resolution in pixels.
    pub input_resolution: [f32; 2],
    /// 1.0 / input_resolution.
    pub input_inv_resolution: [f32; 2],
    /// x: blend_factor, y: clamp_margin, z: sharpen_strength, w: frame_index.
    pub config: [f32; 4],
}

/// Temporal upscale quality preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpscaleQuality {
    /// 50% internal resolution (4× pixel reconstruction).
    Performance,
    /// 67% internal resolution (~2.2× pixel reconstruction).
    Balanced,
    /// 75% internal resolution (~1.8× pixel reconstruction).
    Quality,
    /// 100% — effectively standard TAA (no upscaling).
    Native,
}

impl UpscaleQuality {
    /// Render scale factor (0.0–1.0).
    pub fn render_scale(self) -> f32 {
        match self {
            Self::Performance => 0.50,
            Self::Balanced => 0.67,
            Self::Quality => 0.75,
            Self::Native => 1.00,
        }
    }
}

/// Configuration for the temporal upscaler.
#[derive(Debug, Clone)]
pub struct UpscaleConfig {
    /// Whether temporal upscaling is enabled.
    pub enabled: bool,
    /// Render scale (0.5 = half-res, 1.0 = native). Clamped to [0.25, 1.0].
    pub render_scale: f32,
    /// History blend factor (higher = more stable, lower = more responsive).
    pub blend_factor: f32,
    /// YCoCg AABB clamp margin (larger = less ghosting, more aliasing).
    pub clamp_margin: f32,
    /// RCAS sharpening strength (0 = off).
    pub sharpen_strength: f32,
    /// Number of Halton jitter samples (8 or 16 typical).
    pub jitter_samples: u32,
    /// Jitter scale multiplier.
    pub jitter_scale: f32,
}

impl Default for UpscaleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            render_scale: UpscaleQuality::Balanced.render_scale(),
            blend_factor: 0.95,
            clamp_margin: 0.02,
            sharpen_strength: 0.6,
            jitter_samples: 16,
            jitter_scale: 1.0,
        }
    }
}

impl UpscaleConfig {
    /// Create a config from a quality preset.
    pub fn from_quality(quality: UpscaleQuality) -> Self {
        Self {
            render_scale: quality.render_scale(),
            // Fewer samples is fine at native since there's no upscaling
            jitter_samples: match quality {
                UpscaleQuality::Native => 8,
                _ => 16,
            },
            // Slightly stronger sharpening at lower render scales
            sharpen_strength: match quality {
                UpscaleQuality::Performance => 0.8,
                UpscaleQuality::Balanced => 0.6,
                UpscaleQuality::Quality => 0.5,
                UpscaleQuality::Native => 0.4,
            },
            ..Self::default()
        }
    }

    /// Compute the internal render resolution from display dimensions.
    pub fn internal_resolution(&self, display_width: u32, display_height: u32) -> (u32, u32) {
        let scale = self.render_scale.clamp(0.25, 1.0);
        let w = ((display_width as f32 * scale).round() as u32).max(1);
        let h = ((display_height as f32 * scale).round() as u32).max(1);
        (w, h)
    }
}

/// GPU resources for the temporal upscale pipeline.
///
/// Two compute passes:
/// 1. **Resolve** — upscale + temporal accumulation (input-res → output-res).
/// 2. **Sharpen** — RCAS adaptive sharpening at output resolution.
pub struct TemporalUpscalePass {
    config: UpscaleConfig,
    /// Upscale resolve compute pipeline.
    resolve_pipeline: wgpu::ComputePipeline,
    /// RCAS sharpen compute pipeline.
    sharpen_pipeline: wgpu::ComputePipeline,
    /// Uniform buffer.
    params_buf: wgpu::Buffer,
    /// History texture at output (native) resolution.
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    /// Resolved output texture at output (native) resolution.
    resolved_texture: wgpu::Texture,
    resolved_view: wgpu::TextureView,
    /// Sharpened output texture at output (native) resolution.
    #[allow(dead_code)] // texture must be kept alive for view to remain valid
    sharpened_texture: wgpu::Texture,
    sharpened_view: wgpu::TextureView,
    /// Bind group layout (shared by resolve and sharpen).
    resolve_bgl: wgpu::BindGroupLayout,
    /// Sharpen uses the same BGL shape (current=resolved, output=sharpened).
    sharpen_bgl: wgpu::BindGroupLayout,
    /// Frame counter for jitter.
    frame_index: u32,
    /// Display resolution.
    display_width: u32,
    display_height: u32,
    /// Internal render resolution (derived from display × render_scale).
    render_width: u32,
    render_height: u32,
    /// Sampler (bilinear, used for both passes).
    sampler: wgpu::Sampler,
    /// Cached bind groups.
    cached_resolve_bg: crate::bind_group_cache::CachedBindGroup,
    cached_sharpen_bg: crate::bind_group_cache::CachedBindGroup,
}

impl TemporalUpscalePass {
    /// Create a new temporal upscale pass.
    ///
    /// `display_width`/`display_height` = native output resolution.
    pub fn new(
        device: &wgpu::Device,
        display_width: u32,
        display_height: u32,
        config: &UpscaleConfig,
    ) -> Self {
        let (render_width, render_height) =
            config.internal_resolution(display_width, display_height);
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let make_tex = |label: &str, w: u32, h: u32| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            })
        };

        // All output-sized textures
        let history_texture = make_tex("upscale_history", display_width, display_height);
        let history_view = history_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let resolved_texture = make_tex("upscale_resolved", display_width, display_height);
        let resolved_view = resolved_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sharpened_texture = make_tex("upscale_sharpened", display_width, display_height);
        let sharpened_view = sharpened_texture.create_view(&wgpu::TextureViewDescriptor::default());

        use wgpu::util::DeviceExt;
        let params = UpscaleParams {
            output_resolution: [display_width as f32, display_height as f32],
            output_inv_resolution: [1.0 / display_width as f32, 1.0 / display_height as f32],
            input_resolution: [render_width as f32, render_height as f32],
            input_inv_resolution: [1.0 / render_width as f32, 1.0 / render_height as f32],
            config: [
                config.blend_factor,
                config.clamp_margin,
                config.sharpen_strength,
                0.0,
            ],
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("upscale_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Resolve BGL: current(0), history(1), velocity(2), depth(3), sampler(4), params(5), output(6)
        let resolve_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale_resolve_bgl"),
            entries: &[
                tex_entry(0),
                tex_entry(1),
                tex_entry(2),
                tex_entry(3),
                sampler_entry(4),
                uniform_entry(5),
                storage_entry(6, fmt),
            ],
        });

        // Sharpen BGL: input(0), history(1), velocity(2), depth(3), sampler(4), params(5), output(6)
        // Same shape — the sharpen pass reads `t_current` = resolved and writes to sharpened
        let sharpen_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("upscale_sharpen_bgl"),
            entries: &[
                tex_entry(0),
                tex_entry(1),
                tex_entry(2),
                tex_entry(3),
                sampler_entry(4),
                uniform_entry(5),
                storage_entry(6, fmt),
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal_upscale_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/temporal_upscale.wgsl").into(),
            ),
        });

        let resolve_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("upscale_resolve_pl"),
            bind_group_layouts: &[&resolve_bgl],
            push_constant_ranges: &[],
        });

        let sharpen_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("upscale_sharpen_pl"),
            bind_group_layouts: &[&sharpen_bgl],
            push_constant_ranges: &[],
        });

        let resolve_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("upscale_resolve_pipeline"),
            layout: Some(&resolve_pl),
            module: &shader,
            entry_point: Some("temporal_upscale_resolve"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sharpen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("upscale_rcas_pipeline"),
            layout: Some(&sharpen_pl),
            module: &shader,
            entry_point: Some("upscale_rcas_sharpen"),
            compilation_options: Default::default(),
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("upscale_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            config: config.clone(),
            resolve_pipeline,
            sharpen_pipeline,
            params_buf,
            history_texture,
            history_view,
            resolved_texture,
            resolved_view,
            sharpened_texture,
            sharpened_view,
            resolve_bgl,
            sharpen_bgl,
            frame_index: 0,
            display_width,
            display_height,
            render_width,
            render_height,
            sampler,
            cached_resolve_bg: crate::bind_group_cache::CachedBindGroup::new(),
            cached_sharpen_bg: crate::bind_group_cache::CachedBindGroup::new(),
        }
    }

    /// Get the internal render resolution the scene should render at.
    pub fn render_resolution(&self) -> (u32, u32) {
        (self.render_width, self.render_height)
    }

    /// Get the current jitter offset in pixels (at internal render resolution).
    pub fn current_jitter(&self) -> (f32, f32) {
        super::taa::get_jitter(
            self.frame_index,
            self.config.jitter_samples,
            self.config.jitter_scale,
        )
    }

    /// Get the resolved (pre-sharpen) output view.
    pub fn resolved_view(&self) -> &wgpu::TextureView {
        &self.resolved_view
    }

    /// Get the final sharpened output view (use this for compositing).
    pub fn output_view(&self) -> &wgpu::TextureView {
        &self.sharpened_view
    }

    pub fn config(&self) -> &UpscaleConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: UpscaleConfig) {
        self.config = config;
    }

    /// Advance to next frame.
    pub fn next_frame(&mut self) {
        self.frame_index = self.frame_index.wrapping_add(1);
    }

    pub fn frame_index(&self) -> u32 {
        self.frame_index
    }

    /// Update uniforms for this frame.
    pub fn update_params(&self, queue: &wgpu::Queue) {
        let params = UpscaleParams {
            output_resolution: [self.display_width as f32, self.display_height as f32],
            output_inv_resolution: [
                1.0 / self.display_width as f32,
                1.0 / self.display_height as f32,
            ],
            input_resolution: [self.render_width as f32, self.render_height as f32],
            input_inv_resolution: [
                1.0 / self.render_width as f32,
                1.0 / self.render_height as f32,
            ],
            config: [
                self.config.blend_factor,
                self.config.clamp_margin,
                self.config.sharpen_strength,
                self.frame_index as f32,
            ],
        };
        queue.write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));
    }

    /// Execute the temporal upscale resolve + sharpen passes.
    ///
    /// `current_view` — the scene rendered at internal resolution.
    /// `depth_view`    — depth buffer at internal resolution.
    /// `velocity_view` — motion vectors at internal resolution.
    /// `resource_gen`  — bind group cache generation.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        current_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        velocity_view: &wgpu::TextureView,
        resource_gen: crate::bind_group_cache::Generation,
    ) {
        if !self.config.enabled {
            return;
        }

        // === Resolve pass ===
        if !self.cached_resolve_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("upscale_resolve_bg"),
                layout: &self.resolve_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(current_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(velocity_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&self.resolved_view),
                    },
                ],
            });
            self.cached_resolve_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        let resolve_wg_x = self.display_width.div_ceil(8);
        let resolve_wg_y = self.display_height.div_ceil(8);

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("upscale_resolve"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.resolve_pipeline);
            let bg = self
                .cached_resolve_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(resolve_wg_x, resolve_wg_y, 1);
        }

        // === Sharpen pass ===
        if !self.cached_sharpen_bg.is_valid(resource_gen) {
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("upscale_sharpen_bg"),
                layout: &self.sharpen_bgl,
                entries: &[
                    // Sharpen reads from resolved output
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.resolved_view),
                    },
                    // Unused by sharpen but required by BGL
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.history_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(velocity_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: self.params_buf.as_entire_binding(),
                    },
                    // Sharpen writes to sharpened texture
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&self.sharpened_view),
                    },
                ],
            });
            self.cached_sharpen_bg =
                crate::bind_group_cache::CachedBindGroup::with_bind_group(bg, resource_gen);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("upscale_sharpen"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.sharpen_pipeline);
            let bg = self
                .cached_sharpen_bg
                .get_or_rebuild(resource_gen, || unreachable!());
            pass.set_bind_group(0, bg, &[]);
            pass.dispatch_workgroups(resolve_wg_x, resolve_wg_y, 1);
        }
    }

    /// Copy resolved output to history for next frame's reprojection.
    pub fn copy_to_history(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.resolved_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.history_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.display_width,
                height: self.display_height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Resize when display dimensions change.
    pub fn resize(&mut self, device: &wgpu::Device, display_width: u32, display_height: u32) {
        if self.display_width == display_width && self.display_height == display_height {
            return;
        }
        *self = Self::new(device, display_width, display_height, &self.config);
    }
}

// ============================================================================
// Bind group layout helpers
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscale_params_size() {
        assert_eq!(std::mem::size_of::<UpscaleParams>(), 48);
    }

    #[test]
    fn default_config() {
        let cfg = UpscaleConfig::default();
        assert!(cfg.enabled);
        assert!((cfg.render_scale - 0.67).abs() < 1e-5);
        assert!((cfg.blend_factor - 0.95).abs() < 1e-5);
        assert!((cfg.clamp_margin - 0.02).abs() < 1e-5);
        assert!((cfg.sharpen_strength - 0.6).abs() < 1e-5);
        assert_eq!(cfg.jitter_samples, 16);
    }

    #[test]
    fn quality_presets() {
        let perf = UpscaleConfig::from_quality(UpscaleQuality::Performance);
        assert!((perf.render_scale - 0.50).abs() < 1e-5);
        assert!((perf.sharpen_strength - 0.8).abs() < 1e-5);
        assert_eq!(perf.jitter_samples, 16);

        let balanced = UpscaleConfig::from_quality(UpscaleQuality::Balanced);
        assert!((balanced.render_scale - 0.67).abs() < 1e-5);
        assert!((balanced.sharpen_strength - 0.6).abs() < 1e-5);

        let quality = UpscaleConfig::from_quality(UpscaleQuality::Quality);
        assert!((quality.render_scale - 0.75).abs() < 1e-5);
        assert!((quality.sharpen_strength - 0.5).abs() < 1e-5);

        let native = UpscaleConfig::from_quality(UpscaleQuality::Native);
        assert!((native.render_scale - 1.0).abs() < 1e-5);
        assert_eq!(native.jitter_samples, 8);
    }

    #[test]
    fn internal_resolution_calculation() {
        let cfg = UpscaleConfig::from_quality(UpscaleQuality::Performance);
        let (w, h) = cfg.internal_resolution(1920, 1080);
        assert_eq!(w, 960);
        assert_eq!(h, 540);

        let cfg = UpscaleConfig::from_quality(UpscaleQuality::Balanced);
        let (w, h) = cfg.internal_resolution(1920, 1080);
        assert_eq!(w, 1286);
        assert_eq!(h, 724);

        let cfg = UpscaleConfig::from_quality(UpscaleQuality::Quality);
        let (w, h) = cfg.internal_resolution(1920, 1080);
        assert_eq!(w, 1440);
        assert_eq!(h, 810);

        let cfg = UpscaleConfig::from_quality(UpscaleQuality::Native);
        let (w, h) = cfg.internal_resolution(1920, 1080);
        assert_eq!(w, 1920);
        assert_eq!(h, 1080);
    }

    #[test]
    fn internal_resolution_minimum() {
        let mut cfg = UpscaleConfig::default();
        cfg.render_scale = 0.01; // Absurdly low
        let (w, h) = cfg.internal_resolution(100, 100);
        // Clamped to 0.25 minimum
        assert_eq!(w, 25);
        assert_eq!(h, 25);
    }

    #[test]
    fn parse_temporal_upscale_wgsl() {
        let src = include_str!("../shaders/temporal_upscale.wgsl");
        let module = naga::front::wgsl::parse_str(src);
        assert!(
            module.is_ok(),
            "temporal_upscale.wgsl parse failed: {:?}",
            module.err()
        );
    }

    #[test]
    fn upscale_pass_creation() {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));
        let adapter = match adapter {
            Ok(a) => a,
            Err(_) => {
                eprintln!("skipping GPU test — no adapter");
                return;
            }
        };
        let (device, _queue) =
            match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("upscale_test"),
                ..Default::default()
            })) {
                Ok(pair) => pair,
                Err(_) => {
                    eprintln!("skipping GPU test — no device");
                    return;
                }
            };

        let cfg = UpscaleConfig::from_quality(UpscaleQuality::Balanced);
        let _pass = TemporalUpscalePass::new(&device, 1920, 1080, &cfg);
        // Construction succeeded → pipelines compiled, textures created
    }
}
