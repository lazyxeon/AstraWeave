//! Physically-based bloom with energy-conserving 13-tap downsample and tent upsample.

use wgpu::util::DeviceExt;

/// GPU-side uniform for bloom downsample.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BloomDownsampleParams {
    pub inv_resolution: [f32; 2],
    pub threshold: f32,
    pub soft_knee: f32,
}

/// GPU-side uniform for bloom upsample.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BloomUpsampleParams {
    pub inv_resolution: [f32; 2],
    pub intensity: f32,
    pub _pad: f32,
}

/// Bloom configuration.
#[derive(Debug, Clone)]
pub struct BloomConfig {
    pub enabled: bool,
    /// Brightness threshold for bloom extraction.
    pub threshold: f32,
    /// Smooth threshold transition width.
    pub soft_knee: f32,
    /// Overall bloom intensity.
    pub intensity: f32,
    /// Number of downsample mip levels (typically 5-8).
    pub mip_count: u32,
    /// Per-mip intensity multiplier (index 0 = finest, last = coarsest).
    pub mip_weights: [f32; 8],
}

impl Default for BloomConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 1.0,
            soft_knee: 0.5,
            intensity: 0.3,
            mip_count: 6,
            mip_weights: [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3],
        }
    }
}

/// Manages bloom GPU resources: mip chain textures, compute pipelines.
pub struct BloomPass {
    config: BloomConfig,
    downsample_pipeline: wgpu::ComputePipeline,
    upsample_pipeline: wgpu::ComputePipeline,
    down_bgl: wgpu::BindGroupLayout,
    up_bgl: wgpu::BindGroupLayout,
    /// Mip chain textures (from full res down to smallest).
    mip_textures: Vec<wgpu::Texture>,
    mip_views: Vec<wgpu::TextureView>,
    width: u32,
    height: u32,
}

impl BloomPass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = BloomConfig::default();
        let fmt = wgpu::TextureFormat::Rgba16Float;

        let mut mip_textures = Vec::new();
        let mut mip_views = Vec::new();
        let mut w = width / 2;
        let mut h = height / 2;
        for i in 0..config.mip_count {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("bloom_mip_{i}")),
                size: wgpu::Extent3d {
                    width: w.max(1),
                    height: h.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            mip_views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
            mip_textures.push(tex);
            w /= 2;
            h /= 2;
        }

        let down_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_down_bgl"),
            entries: &[
                bgl_tex(0),
                bgl_sampler(1),
                bgl_uniform(2),
                bgl_storage(3, fmt),
            ],
        });

        let up_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_up_bgl"),
            entries: &[
                bgl_tex(0),
                bgl_tex(1),
                bgl_sampler(2),
                bgl_uniform(3),
                bgl_storage(4, fmt),
            ],
        });

        let down_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_down"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/bloom_downsample.wgsl").into(),
            ),
        });
        let up_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_up"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/bloom_upsample.wgsl").into()),
        });

        let down_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_down_pl"),
            bind_group_layouts: &[&down_bgl],
            push_constant_ranges: &[],
        });
        let up_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom_up_pl"),
            bind_group_layouts: &[&up_bgl],
            push_constant_ranges: &[],
        });

        let downsample_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("bloom_downsample"),
                layout: Some(&down_pl),
                module: &down_shader,
                entry_point: Some("bloom_downsample"),
                compilation_options: Default::default(),
                cache: None,
            });
        let upsample_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bloom_upsample"),
            layout: Some(&up_pl),
            module: &up_shader,
            entry_point: Some("bloom_upsample"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            downsample_pipeline,
            upsample_pipeline,
            down_bgl,
            up_bgl,
            mip_textures,
            mip_views,
            width,
            height,
        }
    }

    pub fn config(&self) -> &BloomConfig {
        &self.config
    }
    pub fn set_config(&mut self, config: BloomConfig) {
        self.config = config;
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn mip_count(&self) -> usize {
        self.mip_views.len()
    }

    /// Get the final bloom texture view (finest mip after upsample).
    pub fn bloom_view(&self) -> Option<&wgpu::TextureView> {
        self.mip_views.first()
    }

    /// Execute the bloom pass: progressive downsample then upsample.
    ///
    /// `scene_view` is the HDR scene texture to extract bloom from.
    /// After execution, `bloom_view()` contains the final bloom result.
    pub fn execute(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        threshold: f32,
        intensity: f32,
    ) {
        if !self.config.enabled || self.mip_views.is_empty() {
            return;
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // --- Downsample chain (scene → mip[0] → mip[1] → ...) ---
        let mut src_width = self.width;
        let mut src_height = self.height;

        for i in 0..self.mip_views.len() {
            let dst_w = (src_width / 2).max(1);
            let dst_h = (src_height / 2).max(1);

            let src_view = if i == 0 {
                scene_view
            } else {
                &self.mip_views[i - 1]
            };

            let params = BloomDownsampleParams {
                inv_resolution: [1.0 / src_width as f32, 1.0 / src_height as f32],
                threshold: if i == 0 { threshold } else { 0.0 }, // only threshold on first pass
                soft_knee: self.config.soft_knee,
            };
            let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bloom_down_params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_down_bg"),
                layout: &self.down_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.mip_views[i]),
                    },
                ],
            });

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bloom_downsample"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((dst_w + 7) / 8, (dst_h + 7) / 8, 1);

            src_width = dst_w;
            src_height = dst_h;
        }

        // --- Upsample chain (mip[n-1] → mip[n-2] → ... → mip[0]) ---
        if self.mip_views.len() >= 2 {
            for i in (0..self.mip_views.len() - 1).rev() {
                let coarse_view = &self.mip_views[i + 1];
                let fine_view = &self.mip_views[i];
                let fine_w = (self.width / 2u32.pow(i as u32 + 1)).max(1);
                let fine_h = (self.height / 2u32.pow(i as u32 + 1)).max(1);

                let weight = intensity * self.config.mip_weights.get(i).copied().unwrap_or(0.5);
                let params = BloomUpsampleParams {
                    inv_resolution: [1.0 / fine_w as f32, 1.0 / fine_h as f32],
                    intensity: weight,
                    _pad: 0.0,
                };
                let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("bloom_up_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bloom_up_bg"),
                    layout: &self.up_bgl,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(coarse_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(fine_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: params_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(fine_view),
                        },
                    ],
                });

                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bloom_upsample"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.upsample_pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups((fine_w + 7) / 8, (fine_h + 7) / 8, 1);
            }
        }
    }

    /// Get the final bloom output view (same as bloom_view, alias for consistency).
    pub fn output_view(&self) -> &wgpu::TextureView {
        self.mip_views
            .first()
            .expect("Bloom must have at least 1 mip")
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

fn bgl_tex(binding: u32) -> wgpu::BindGroupLayoutEntry {
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
fn bgl_storage(binding: u32, format: wgpu::TextureFormat) -> wgpu::BindGroupLayoutEntry {
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
    fn bloom_config_default() {
        let c = BloomConfig::default();
        assert!(c.enabled);
        assert_eq!(c.mip_count, 6);
        assert!((c.threshold - 1.0).abs() < 1e-5);
    }

    #[test]
    fn bloom_params_sizes() {
        assert_eq!(std::mem::size_of::<BloomDownsampleParams>(), 16);
        assert_eq!(std::mem::size_of::<BloomUpsampleParams>(), 16);
    }

    #[test]
    fn bloom_pass_creation() {
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

        let pass = BloomPass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
        assert_eq!(pass.mip_count(), 6);
        assert!(pass.bloom_view().is_some());
    }
}
