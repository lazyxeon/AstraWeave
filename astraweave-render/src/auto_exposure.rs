//! Auto-exposure via luminance histogram with temporal adaptation.
//!
//! Two-pass compute system:
//! 1. Histogram: builds a 256-bin luminance histogram from the HDR image
//! 2. Average: computes percentile-trimmed average luminance, adapts exposure over time

/// GPU-side uniform for auto-exposure.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExposureParams {
    pub resolution: [f32; 2],
    pub inv_resolution: [f32; 2],
    pub min_log_lum: f32,
    pub max_log_lum: f32,
    pub time_delta: f32,
    pub adaptation_speed: f32,
    pub low_percentile: f32,
    pub high_percentile: f32,
    pub target_exposure: f32,
    pub _pad: f32,
}

impl Default for ExposureParams {
    fn default() -> Self {
        Self {
            resolution: [1920.0, 1080.0],
            inv_resolution: [1.0 / 1920.0, 1.0 / 1080.0],
            min_log_lum: -10.0,
            max_log_lum: 2.0,
            time_delta: 1.0 / 60.0,
            adaptation_speed: 3.0,
            low_percentile: 0.1,
            high_percentile: 0.95,
            target_exposure: 0.0,
            _pad: 0.0,
        }
    }
}

/// Auto-exposure configuration.
#[derive(Debug, Clone)]
pub struct AutoExposureConfig {
    pub enabled: bool,
    pub min_log_luminance: f32,
    pub max_log_luminance: f32,
    pub adaptation_speed: f32,
    pub low_percentile: f32,
    pub high_percentile: f32,
    /// Manual exposure override (0 = auto).
    pub manual_exposure: f32,
}

impl Default for AutoExposureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_log_luminance: -10.0,
            max_log_luminance: 2.0,
            adaptation_speed: 3.0,
            low_percentile: 0.1,
            high_percentile: 0.95,
            manual_exposure: 0.0,
        }
    }
}

/// Manages auto-exposure GPU resources.
pub struct AutoExposurePass {
    config: AutoExposureConfig,
    histogram_pipeline: wgpu::ComputePipeline,
    average_pipeline: wgpu::ComputePipeline,
    params_buf: wgpu::Buffer,
    /// 256-bin histogram (u32 per bin, storage buffer with atomics).
    histogram_buf: wgpu::Buffer,
    /// Exposure data: [current_ev, target_ev].
    exposure_buf: wgpu::Buffer,
    bgl: wgpu::BindGroupLayout,
    width: u32,
    height: u32,
}

impl AutoExposurePass {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let config = AutoExposureConfig::default();

        use wgpu::util::DeviceExt;
        let params = ExposureParams {
            resolution: [width as f32, height as f32],
            inv_resolution: [1.0 / width as f32, 1.0 / height as f32],
            ..ExposureParams::default()
        };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("exposure_params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let histogram_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("luminance_histogram"),
            size: 256 * 4, // 256 u32 bins
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let exposure_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("exposure_data"),
            contents: bytemuck::cast_slice(&[0.0f32, 0.0f32]),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("auto_exposure_bgl"),
            entries: &[
                // 0: HDR texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // 1: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // 2: params
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 3: histogram (atomic storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // 4: exposure data (storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("auto_exposure_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/auto_exposure.wgsl").into()),
        });

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("auto_exposure_pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let histogram_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("histogram_pass"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("histogram_pass"),
            compilation_options: Default::default(),
            cache: None,
        });

        let average_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("average_pass"),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some("average_pass"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            config,
            histogram_pipeline,
            average_pipeline,
            params_buf,
            histogram_buf,
            exposure_buf,
            bgl,
            width,
            height,
        }
    }

    pub fn config(&self) -> &AutoExposureConfig {
        &self.config
    }
    pub fn set_config(&mut self, config: AutoExposureConfig) {
        self.config = config;
    }

    /// Get the exposure storage buffer (for reading current exposure value in tonemapping).
    pub fn exposure_buffer(&self) -> &wgpu::Buffer {
        &self.exposure_buf
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Execute the auto-exposure pipeline: histogram → average.
    ///
    /// Clears histogram, builds luminance bins from the HDR scene, then computes
    /// the percentile-trimmed average exposure.
    pub fn execute(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        hdr_view: &wgpu::TextureView,
    ) {
        if !self.config.enabled {
            return;
        }

        // Clear histogram to zero
        encoder.clear_buffer(&self.histogram_buf, 0, None);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("exposure_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("auto_exposure_bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.histogram_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.exposure_buf.as_entire_binding(),
                },
            ],
        });

        // Pass 1: Build histogram
        {
            let wg_x = (self.width + 15) / 16;
            let wg_y = (self.height + 15) / 16;
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("histogram_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.histogram_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        // Pass 2: Compute average exposure
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("average_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.average_pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
    }

    /// Get current exposure EV value (for CPU-side query — reads last GPU value).
    /// Note: This returns the manual override when auto-exposure is disabled.
    pub fn current_ev(&self) -> f32 {
        if self.config.enabled {
            0.0 // GPU-side value; CPU readback would require async map
        } else {
            self.config.manual_exposure
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }
        *self = Self::new(device, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposure_params_size() {
        assert_eq!(std::mem::size_of::<ExposureParams>(), 48);
    }

    #[test]
    fn auto_exposure_config_default() {
        let c = AutoExposureConfig::default();
        assert!(c.enabled);
        assert_eq!(c.manual_exposure, 0.0);
        assert_eq!(c.adaptation_speed, 3.0);
    }

    #[test]
    fn auto_exposure_pass_creation() {
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

        let pass = AutoExposurePass::new(&device, 1920, 1080);
        assert_eq!(pass.dimensions(), (1920, 1080));
    }
}
