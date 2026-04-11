//! Shared Hi-Z (Hierarchical Depth) min-depth pyramid for screen-space tracing.
//!
//! Builds a min-depth mip chain from the depth buffer. Used by SSR and SSGI
//! to accelerate ray traversal by skipping empty space at coarse mip levels.
//!
//! The pyramid stores the **minimum** depth in each 2×2 block, so the tracer
//! knows that if the ray depth is less than the stored min-depth at a given
//! mip level, there is no surface intersection in that region and the ray
//! can safely advance to the next cell.

/// Min-depth Hi-Z pyramid for screen-space ray tracing (SSR, SSGI).
pub struct HizPyramid {
    copy_pipeline: wgpu::ComputePipeline,
    downsample_pipeline: wgpu::ComputePipeline,
    bgl: wgpu::BindGroupLayout,
    #[allow(dead_code)] // texture must be kept alive for views to remain valid
    hiz_texture: wgpu::Texture,
    /// Per-mip views for downsample dispatch (single-mip views).
    #[allow(dead_code)] // views must be kept alive for bind groups to remain valid
    mip_views: Vec<wgpu::TextureView>,
    /// Full mip-chain view for sampling in SSR/SSGI shaders.
    full_view: wgpu::TextureView,
    /// bind_groups[0]: copy — src depth → mip 0.
    /// bind_groups[1..]: downsample — mip[i-1] → mip[i].
    bind_groups: Vec<wgpu::BindGroup>,
    mip_count: u32,
    width: u32,
    height: u32,
}

impl HizPyramid {
    /// Create a new Hi-Z pyramid for the given resolution.
    ///
    /// `src_depth_view` is the view of the source depth texture (R32Float or
    /// similar filterable float format) that will be copied into mip 0 during
    /// [`build`].
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        src_depth_view: &wgpu::TextureView,
    ) -> Self {
        let mip_count = (width.max(height) as f32).log2().ceil() as u32;
        let fmt = wgpu::TextureFormat::R32Float;

        let hiz_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hiz_min_pyramid"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: fmt,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let mip_views: Vec<wgpu::TextureView> = (0..mip_count)
            .map(|mip| {
                hiz_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some(&format!("hiz_min_mip_{mip}")),
                    format: Some(fmt),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                    usage: None,
                })
            })
            .collect();

        let full_view = hiz_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("hiz_min_full"),
            ..Default::default()
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hiz_pyramid_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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

        // Bind group 0: copy — src_depth → hiz mip 0
        let mut bind_groups = vec![device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hiz_copy_bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mip_views[0]),
                },
            ],
        })];

        // Bind groups 1..N-1: downsample — mip[i] → mip[i+1]
        for i in 0..mip_count.saturating_sub(1) {
            bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("hiz_ds_bg_{i}_{}", i + 1)),
                layout: &bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&mip_views[i as usize]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&mip_views[(i + 1) as usize]),
                    },
                ],
            }));
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hiz_pyramid_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/hiz_pyramid.wgsl").into()),
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hiz_pyramid_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let copy_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("hiz_copy_pipeline"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("copy_depth"),
            compilation_options: Default::default(),
            cache: None,
        });

        let downsample_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("hiz_downsample_pipeline"),
                layout: Some(&pl),
                module: &shader,
                entry_point: Some("downsample"),
                compilation_options: Default::default(),
                cache: None,
            });

        Self {
            copy_pipeline,
            downsample_pipeline,
            bgl,
            hiz_texture,
            mip_views,
            full_view,
            bind_groups,
            mip_count,
            width,
            height,
        }
    }

    /// Returns the full mip-chain view for SSR/SSGI shader sampling.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.full_view
    }

    /// Number of mip levels in the pyramid.
    pub fn mip_count(&self) -> u32 {
        self.mip_count
    }

    /// Resolution of the pyramid (matches source depth).
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Build the Hi-Z pyramid from the current frame's depth buffer.
    ///
    /// 1. Copies the source depth into mip 0 (using the copy compute shader).
    /// 2. Downsamples each mip level using min-depth reduction.
    pub fn build(&self, encoder: &mut wgpu::CommandEncoder) {
        // Pass 0: copy source depth → mip 0
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("hiz_copy_depth"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.copy_pipeline);
            pass.set_bind_group(0, &self.bind_groups[0], &[]);
            pass.dispatch_workgroups(self.width.div_ceil(8), self.height.div_ceil(8), 1);
        }

        // Passes 1..N: downsample mip[i] → mip[i+1]
        for i in 0..self.mip_count.saturating_sub(1) {
            let mip_w = (self.width >> (i + 1)).max(1);
            let mip_h = (self.height >> (i + 1)).max(1);

            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("hiz_downsample"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.downsample_pipeline);
            // bind_groups[i+1] maps mip[i] → mip[i+1]
            pass.set_bind_group(0, &self.bind_groups[(i + 1) as usize], &[]);
            pass.dispatch_workgroups(mip_w.div_ceil(8), mip_h.div_ceil(8), 1);
        }
    }

    /// Recreate the pyramid for a new resolution and/or depth view.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        src_depth_view: &wgpu::TextureView,
    ) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height, src_depth_view);
    }

    /// Bind group layout (exposed for other systems that need to match).
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bgl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hiz_mip_count_1920x1080() {
        let mip_count = (1920_u32.max(1080) as f32).log2().ceil() as u32;
        assert_eq!(mip_count, 11); // 1920 → ceil(log2(1920)) = 11
    }

    #[test]
    fn hiz_mip_count_power_of_two() {
        let mip_count = (1024_u32.max(1024) as f32).log2().ceil() as u32;
        assert_eq!(mip_count, 10); // 1024 → log2(1024) = 10
    }

    #[test]
    fn hiz_pyramid_creation() {
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

        // Create a dummy R32Float depth source texture
        let depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test_depth"),
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let pyramid = HizPyramid::new(&device, 1920, 1080, &depth_view);
        assert_eq!(pyramid.dimensions(), (1920, 1080));
        assert_eq!(pyramid.mip_count(), 11);
    }
}
